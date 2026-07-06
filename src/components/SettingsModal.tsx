import { useState, useEffect } from 'react'
import {
  claimSetupToken,
  getSimpleFINStatus,
  disconnectSimpleFIN,
  getAccounts,
  setDebtTerms,
  saveLlmConfig,
  saveSyncSettings,
  saveUiPreferences,
  getSettings,
  type Account,
  type SimpleFINStatusResponse,
} from '../lib/tauri-commands'

interface SettingsModalProps {
  isOpen: boolean
  onClose: () => void
}

type SettingsTab = 'simplefin' | 'debt-terms' | 'llm-config' | 'sync-settings' | 'ui-prefs' | 'about'

export function SettingsModal({ isOpen, onClose }: SettingsModalProps) {
  const [tab, setTab] = useState<SettingsTab>('simplefin')
  const [setupToken, setSetupToken] = useState('')
  const [loading, setLoading] = useState(false)
  const [message, setMessage] = useState('')
  const [error, setError] = useState('')
  const [simplefinStatus, setSimplefinStatus] = useState<SimpleFINStatusResponse>({
    connected: false,
  })
  const [accounts, setAccounts] = useState<Account[]>([])
  const [debtTermsForm, setDebtTermsForm] = useState<Record<string, { apr: string; minPayment: string }>>({})

  // LLM Configuration state
  const [ollamaUrl, setOllamaUrl] = useState('http://localhost:11434')
  const [apiKey, setApiKey] = useState('')
  const [llmModel, setLlmModel] = useState('mistral')
  const [useLocalFirst, setUseLocalFirst] = useState(true)

  // Sync Settings state
  const [syncFrequency, setSyncFrequency] = useState<'manual' | 'on-open' | '12h' | '24h'>('on-open')
  const [backfillDays, setBackfillDays] = useState('90')
  const [enableBackgroundSync, setEnableBackgroundSync] = useState(false)

  // UI Preferences state
  const [theme, setTheme] = useState<'light' | 'dark' | 'auto'>('dark')
  const [currency, setCurrency] = useState('USD')

  useEffect(() => {
    if (isOpen) {
      loadSimplefinStatus()
      loadAccounts()
      loadSettings()
    }
  }, [isOpen])

  async function loadSettings() {
    try {
      const settings = await getSettings()

      if (settings.llm_config) {
        setOllamaUrl(settings.llm_config.ollama_url)
        setLlmModel(settings.llm_config.llm_model)
        setUseLocalFirst(settings.llm_config.use_local_first)
      }

      if (settings.sync_settings) {
        setSyncFrequency(settings.sync_settings.sync_frequency)
        setBackfillDays(settings.sync_settings.backfill_days.toString())
        setEnableBackgroundSync(settings.sync_settings.enable_background_sync)
      }

      if (settings.ui_preferences) {
        setTheme(settings.ui_preferences.theme)
        setCurrency(settings.ui_preferences.currency)
      }

      // Also load from localStorage for fallback (UI preferences can be stored locally)
      const saved = localStorage.getItem('momentum_settings')
      if (saved) {
        const localSettings = JSON.parse(saved)
        if (!settings.ui_preferences && localSettings.theme) {
          setTheme(localSettings.theme)
        }
        if (!settings.ui_preferences && localSettings.currency) {
          setCurrency(localSettings.currency)
        }
      }
    } catch (err) {
      console.error('Failed to load settings:', err)
      // Fallback to localStorage if backend fails
      const saved = localStorage.getItem('momentum_settings')
      if (saved) {
        const settings = JSON.parse(saved)
        if (settings.ollama_url) setOllamaUrl(settings.ollama_url)
        if (settings.llm_model) setLlmModel(settings.llm_model)
        if (settings.use_local_first !== undefined) setUseLocalFirst(settings.use_local_first)
        if (settings.sync_frequency) setSyncFrequency(settings.sync_frequency)
        if (settings.backfill_days) setBackfillDays(settings.backfill_days)
        if (settings.enable_background_sync !== undefined) setEnableBackgroundSync(settings.enable_background_sync)
        if (settings.theme) setTheme(settings.theme)
        if (settings.currency) setCurrency(settings.currency)
      }
    }
  }

  async function loadSimplefinStatus() {
    try {
      const status = await getSimpleFINStatus()
      setSimplefinStatus(status)
    } catch (err) {
      console.error('Failed to load SimpleFIN status:', err)
    }
  }

  async function loadAccounts() {
    try {
      const accts = await getAccounts()
      setAccounts(accts)
      // Initialize debt terms form with empty values
      const form: Record<string, { apr: string; minPayment: string }> = {}
      accts.forEach((acc) => {
        form[acc.id] = { apr: '', minPayment: '' }
      })
      setDebtTermsForm(form)
    } catch (err) {
      console.error('Failed to load accounts:', err)
    }
  }

  async function handleClaimToken() {
    if (!setupToken.trim()) {
      setError('Please enter a setup token')
      return
    }

    setLoading(true)
    setError('')
    setMessage('')

    try {
      await claimSetupToken({ setup_token: setupToken })
      setMessage('SimpleFIN connected successfully!')
      setSetupToken('')
      setTimeout(() => {
        loadSimplefinStatus()
        loadAccounts()
      }, 500)
    } catch (err: any) {
      setError(err?.message || 'Failed to claim setup token')
    } finally {
      setLoading(false)
    }
  }

  async function handleDisconnect() {
    if (!window.confirm('Are you sure? This will disconnect SimpleFIN.')) {
      return
    }

    setLoading(true)
    setError('')
    setMessage('')

    try {
      await disconnectSimpleFIN()
      setMessage('SimpleFIN disconnected')
      setTimeout(() => {
        loadSimplefinStatus()
      }, 500)
    } catch (err: any) {
      setError(err?.message || 'Failed to disconnect')
    } finally {
      setLoading(false)
    }
  }

  async function handleSaveDebtTerms(accountId: string) {
    const form = debtTermsForm[accountId]
    if (!form || !form.apr) {
      setError('Please enter an APR')
      return
    }

    const apr = parseFloat(form.apr) / 100 // Convert percentage to decimal
    if (apr < 0 || apr > 1) {
      setError('APR must be between 0 and 100')
      return
    }

    const minPayment = form.minPayment ? parseFloat(form.minPayment) : undefined

    setLoading(true)
    setError('')
    setMessage('')

    try {
      await setDebtTerms({
        account_id: accountId,
        interest_rate: apr,
        minimum_payment: minPayment,
      })
      setMessage(`Debt terms saved for ${accounts.find((a) => a.id === accountId)?.name}`)
    } catch (err: any) {
      setError(err?.message || 'Failed to save debt terms')
    } finally {
      setLoading(false)
    }
  }

  async function handleSaveLlmConfig() {
    if (!ollamaUrl.trim()) {
      setError('Please enter an Ollama URL')
      return
    }

    setLoading(true)
    setError('')
    setMessage('')

    try {
      await saveLlmConfig({
        ollama_url: ollamaUrl,
        llm_model: llmModel,
        use_local_first: useLocalFirst,
        api_key: apiKey || undefined,
      })
      setMessage('LLM configuration saved successfully')
    } catch (err: any) {
      setError(err?.message || 'Failed to save LLM configuration')
    } finally {
      setLoading(false)
    }
  }

  async function handleSaveSyncSettings() {
    const days = parseInt(backfillDays)
    if (isNaN(days) || days < 1 || days > 3650) {
      setError('Backfill days must be between 1 and 3650')
      return
    }

    setLoading(true)
    setError('')
    setMessage('')

    try {
      await saveSyncSettings({
        sync_frequency: syncFrequency,
        backfill_days: days,
        enable_background_sync: enableBackgroundSync,
      })
      setMessage('Sync settings saved successfully')
    } catch (err: any) {
      setError(err?.message || 'Failed to save sync settings')
    } finally {
      setLoading(false)
    }
  }

  async function handleSaveUiPreferences() {
    setLoading(true)
    setError('')
    setMessage('')

    try {
      await saveUiPreferences({
        theme,
        currency,
      })

      // Apply theme to document if needed
      if (theme === 'dark' || (theme === 'auto' && window.matchMedia('(prefers-color-scheme: dark)').matches)) {
        document.documentElement.classList.add('dark')
      } else {
        document.documentElement.classList.remove('dark')
      }

      // Also store in localStorage for immediate effect and fallback
      localStorage.setItem('momentum_ui_prefs', JSON.stringify({ theme, currency }))

      setMessage('UI preferences saved successfully')
    } catch (err: any) {
      setError(err?.message || 'Failed to save UI preferences')
    } finally {
      setLoading(false)
    }
  }

  if (!isOpen) return null

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-gray-800 rounded-lg shadow-xl max-w-2xl w-full max-h-[90vh] overflow-hidden flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-gray-700">
          <h2 className="text-2xl font-bold text-white">Settings</h2>
          <button
            onClick={onClose}
            className="text-gray-400 hover:text-white transition-colors"
          >
            <svg
              className="w-6 h-6"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        </div>

        {/* Tabs */}
        <div className="flex flex-wrap border-b border-gray-700 px-6 overflow-x-auto">
          <button
            onClick={() => setTab('simplefin')}
            className={`px-4 py-2 border-b-2 font-medium transition-colors whitespace-nowrap ${
              tab === 'simplefin'
                ? 'border-blue-500 text-blue-400'
                : 'border-transparent text-gray-400 hover:text-white'
            }`}
          >
            SimpleFIN
          </button>
          <button
            onClick={() => setTab('debt-terms')}
            className={`px-4 py-2 border-b-2 font-medium transition-colors whitespace-nowrap ${
              tab === 'debt-terms'
                ? 'border-blue-500 text-blue-400'
                : 'border-transparent text-gray-400 hover:text-white'
            }`}
          >
            Debt Terms
          </button>
          <button
            onClick={() => setTab('llm-config')}
            className={`px-4 py-2 border-b-2 font-medium transition-colors whitespace-nowrap ${
              tab === 'llm-config'
                ? 'border-blue-500 text-blue-400'
                : 'border-transparent text-gray-400 hover:text-white'
            }`}
          >
            LLM Config
          </button>
          <button
            onClick={() => setTab('sync-settings')}
            className={`px-4 py-2 border-b-2 font-medium transition-colors whitespace-nowrap ${
              tab === 'sync-settings'
                ? 'border-blue-500 text-blue-400'
                : 'border-transparent text-gray-400 hover:text-white'
            }`}
          >
            Sync Settings
          </button>
          <button
            onClick={() => setTab('ui-prefs')}
            className={`px-4 py-2 border-b-2 font-medium transition-colors whitespace-nowrap ${
              tab === 'ui-prefs'
                ? 'border-blue-500 text-blue-400'
                : 'border-transparent text-gray-400 hover:text-white'
            }`}
          >
            UI Preferences
          </button>
          <button
            onClick={() => setTab('about')}
            className={`px-4 py-2 border-b-2 font-medium transition-colors whitespace-nowrap ${
              tab === 'about'
                ? 'border-blue-500 text-blue-400'
                : 'border-transparent text-gray-400 hover:text-white'
            }`}
          >
            About
          </button>
        </div>

        {/* Content */}
        <div className="overflow-y-auto flex-1 px-6 py-4">
          {/* Messages */}
          {error && (
            <div className="mb-4 p-3 bg-red-900/30 border border-red-500 rounded text-red-300 text-sm">
              {error}
            </div>
          )}
          {message && (
            <div className="mb-4 p-3 bg-green-900/30 border border-green-500 rounded text-green-300 text-sm">
              {message}
            </div>
          )}

          {/* SimpleFIN Tab */}
          {tab === 'simplefin' && (
            <div className="space-y-4">
              <h3 className="text-lg font-semibold text-white">SimpleFIN Integration</h3>

              {simplefinStatus.connected ? (
                <div className="space-y-3">
                  <div className="p-3 bg-green-900/30 border border-green-500 rounded">
                    <p className="text-green-300 font-medium">✓ Connected</p>
                    {simplefinStatus.account_count && (
                      <p className="text-green-300 text-sm mt-1">
                        {simplefinStatus.account_count} account{simplefinStatus.account_count !== 1 ? 's' : ''} linked
                      </p>
                    )}
                  </div>
                  <button
                    onClick={handleDisconnect}
                    disabled={loading}
                    className="w-full px-4 py-2 bg-red-600 hover:bg-red-700 disabled:opacity-50 text-white rounded font-medium transition-colors"
                  >
                    {loading ? 'Disconnecting...' : 'Disconnect'}
                  </button>
                </div>
              ) : (
                <div className="space-y-3">
                  <p className="text-gray-400 text-sm">
                    Paste your SimpleFIN setup token to connect your financial accounts. Get a setup token at{' '}
                    <a
                      href="https://simplefin.com/"
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-blue-400 hover:underline"
                    >
                      simplefin.com
                    </a>
                    .
                  </p>
                  <input
                    type="text"
                    placeholder="https://simplefin.com/sync/setup/..."
                    value={setupToken}
                    onChange={(e) => setSetupToken(e.target.value)}
                    disabled={loading}
                    className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white placeholder-gray-500 focus:outline-none focus:border-blue-500"
                  />
                  <button
                    onClick={handleClaimToken}
                    disabled={loading || !setupToken.trim()}
                    className="w-full px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white rounded font-medium transition-colors"
                  >
                    {loading ? 'Connecting...' : 'Connect SimpleFIN'}
                  </button>
                </div>
              )}
            </div>
          )}

          {/* Debt Terms Tab */}
          {tab === 'debt-terms' && (
            <div className="space-y-4">
              <h3 className="text-lg font-semibold text-white">Debt Terms</h3>
              {accounts.length === 0 ? (
                <p className="text-gray-400 text-sm">No accounts available. Connect SimpleFIN first.</p>
              ) : (
                <div className="space-y-4">
                  {accounts.map((account) => (
                    <div key={account.id} className="border border-gray-700 rounded p-4 space-y-3">
                      <h4 className="font-medium text-white">{account.name}</h4>
                      <div className="grid grid-cols-2 gap-3">
                        <div>
                          <label className="block text-sm text-gray-400 mb-1">APR (%)</label>
                          <input
                            type="number"
                            placeholder="21.99"
                            step={0.01}
                            min={0}
                            max={100}
                            value={debtTermsForm[account.id]?.apr || ''}
                            onChange={(e) =>
                              setDebtTermsForm((prev) => ({
                                ...prev,
                                [account.id]: { ...prev[account.id], apr: e.target.value },
                              }))
                            }
                            disabled={loading}
                            className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white placeholder-gray-500 focus:outline-none focus:border-blue-500"
                          />
                        </div>
                        <div>
                          <label className="block text-sm text-gray-400 mb-1">Min Payment ($)</label>
                          <input
                            type="number"
                            placeholder="100"
                            step={0.01}
                            min={0}
                            value={debtTermsForm[account.id]?.minPayment || ''}
                            onChange={(e) =>
                              setDebtTermsForm((prev) => ({
                                ...prev,
                                [account.id]: { ...prev[account.id], minPayment: e.target.value },
                              }))
                            }
                            disabled={loading}
                            className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white placeholder-gray-500 focus:outline-none focus:border-blue-500"
                          />
                        </div>
                      </div>
                      <button
                        onClick={() => handleSaveDebtTerms(account.id)}
                        disabled={loading}
                        className="w-full px-3 py-2 bg-green-600 hover:bg-green-700 disabled:opacity-50 text-white rounded font-medium text-sm transition-colors"
                      >
                        {loading ? 'Saving...' : 'Save'}
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {/* LLM Configuration Tab */}
          {tab === 'llm-config' && (
            <div className="space-y-4">
              <h3 className="text-lg font-semibold text-white">LLM Configuration</h3>
              <p className="text-gray-400 text-sm">
                Configure transaction categorization. Momentum uses local Ollama for privacy, with API fallback.
              </p>

              <div className="space-y-3">
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Ollama URL</label>
                  <input
                    type="text"
                    placeholder="http://localhost:11434"
                    value={ollamaUrl}
                    onChange={(e) => setOllamaUrl(e.target.value)}
                    disabled={loading}
                    className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white placeholder-gray-500 focus:outline-none focus:border-blue-500"
                  />
                  <p className="text-xs text-gray-500 mt-1">Local Ollama instance endpoint</p>
                </div>

                <div>
                  <label className="block text-sm text-gray-400 mb-1">Model</label>
                  <select
                    value={llmModel}
                    onChange={(e) => setLlmModel(e.target.value)}
                    disabled={loading}
                    className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white focus:outline-none focus:border-blue-500"
                  >
                    <option value="mistral">Mistral</option>
                    <option value="llama2">Llama 2</option>
                    <option value="neural-chat">Neural Chat</option>
                    <option value="openchat">Open Chat</option>
                  </select>
                </div>

                <div>
                  <label className="block text-sm text-gray-400 mb-1">API Key (optional)</label>
                  <input
                    type="password"
                    placeholder="sk-..."
                    value={apiKey}
                    onChange={(e) => setApiKey(e.target.value)}
                    disabled={loading}
                    className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white placeholder-gray-500 focus:outline-none focus:border-blue-500"
                  />
                  <p className="text-xs text-gray-500 mt-1">For API fallback (Claude, etc.)</p>
                </div>

                <div className="flex items-center space-x-2 pt-2">
                  <input
                    type="checkbox"
                    id="use-local-first"
                    checked={useLocalFirst}
                    onChange={(e) => setUseLocalFirst(e.target.checked)}
                    disabled={loading}
                    className="w-4 h-4 rounded border-gray-600 bg-gray-700 cursor-pointer"
                  />
                  <label htmlFor="use-local-first" className="text-sm text-gray-400 cursor-pointer">
                    Use local Ollama first, fall back to API if unavailable
                  </label>
                </div>

                <button
                  onClick={handleSaveLlmConfig}
                  disabled={loading}
                  className="w-full px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white rounded font-medium transition-colors"
                >
                  {loading ? 'Saving...' : 'Save LLM Configuration'}
                </button>
              </div>
            </div>
          )}

          {/* Sync Settings Tab */}
          {tab === 'sync-settings' && (
            <div className="space-y-4">
              <h3 className="text-lg font-semibold text-white">Sync Settings</h3>
              <p className="text-gray-400 text-sm">
                Control how SimpleFIN data is synced and categorized.
              </p>

              <div className="space-y-3">
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Sync Frequency</label>
                  <select
                    value={syncFrequency}
                    onChange={(e) => setSyncFrequency(e.target.value as any)}
                    disabled={loading}
                    className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white focus:outline-none focus:border-blue-500"
                  >
                    <option value="manual">Manual only</option>
                    <option value="on-open">On app open (if &gt;24h)</option>
                    <option value="12h">Every 12 hours</option>
                    <option value="24h">Every 24 hours</option>
                  </select>
                </div>

                <div>
                  <label className="block text-sm text-gray-400 mb-1">Backfill Range (days)</label>
                  <input
                    type="number"
                    placeholder="90"
                    min="1"
                    max="3650"
                    value={backfillDays}
                    onChange={(e) => setBackfillDays(e.target.value)}
                    disabled={loading}
                    className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white placeholder-gray-500 focus:outline-none focus:border-blue-500"
                  />
                  <p className="text-xs text-gray-500 mt-1">How far back to sync transactions on each sync</p>
                </div>

                <div className="flex items-center space-x-2 pt-2">
                  <input
                    type="checkbox"
                    id="enable-background-sync"
                    checked={enableBackgroundSync}
                    onChange={(e) => setEnableBackgroundSync(e.target.checked)}
                    disabled={loading}
                    className="w-4 h-4 rounded border-gray-600 bg-gray-700 cursor-pointer"
                  />
                  <label htmlFor="enable-background-sync" className="text-sm text-gray-400 cursor-pointer">
                    Enable background sync (app continues running in background)
                  </label>
                </div>

                <button
                  onClick={handleSaveSyncSettings}
                  disabled={loading}
                  className="w-full px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white rounded font-medium transition-colors"
                >
                  {loading ? 'Saving...' : 'Save Sync Settings'}
                </button>
              </div>
            </div>
          )}

          {/* UI Preferences Tab */}
          {tab === 'ui-prefs' && (
            <div className="space-y-4">
              <h3 className="text-lg font-semibold text-white">UI Preferences</h3>
              <p className="text-gray-400 text-sm">
                Customize the appearance and display format.
              </p>

              <div className="space-y-3">
                <div>
                  <label className="block text-sm text-gray-400 mb-1">Theme</label>
                  <select
                    value={theme}
                    onChange={(e) => setTheme(e.target.value as any)}
                    disabled={loading}
                    className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white focus:outline-none focus:border-blue-500"
                  >
                    <option value="light">Light</option>
                    <option value="dark">Dark</option>
                    <option value="auto">Auto (system preference)</option>
                  </select>
                </div>

                <div>
                  <label className="block text-sm text-gray-400 mb-1">Currency</label>
                  <select
                    value={currency}
                    onChange={(e) => setCurrency(e.target.value)}
                    disabled={loading}
                    className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white focus:outline-none focus:border-blue-500"
                  >
                    <option value="USD">USD ($)</option>
                    <option value="EUR">EUR (€)</option>
                    <option value="GBP">GBP (£)</option>
                    <option value="CAD">CAD (C$)</option>
                    <option value="AUD">AUD (A$)</option>
                    <option value="JPY">JPY (¥)</option>
                  </select>
                </div>

                <button
                  onClick={handleSaveUiPreferences}
                  disabled={loading}
                  className="w-full px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white rounded font-medium transition-colors"
                >
                  {loading ? 'Saving...' : 'Save UI Preferences'}
                </button>
              </div>
            </div>
          )}

          {/* About Tab */}
          {tab === 'about' && (
            <div className="space-y-4">
              <h3 className="text-lg font-semibold text-white">About Momentum</h3>
              <div className="space-y-2 text-gray-400 text-sm">
                <p>
                  <strong className="text-white">Version:</strong> 0.0.5
                </p>
                <p>
                  <strong className="text-white">Description:</strong> Local-first budgeting app with cash flow momentum metrics
                </p>
                <p className="pt-2">
                  Momentum uses SimpleFIN to securely sync your financial accounts. Transaction categorization uses local Ollama AI with optional Claude API fallback. Your data is stored locally on your device.
                </p>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="border-t border-gray-700 px-6 py-3 bg-gray-750 flex justify-end">
          <button
            onClick={onClose}
            className="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded font-medium transition-colors"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  )
}
