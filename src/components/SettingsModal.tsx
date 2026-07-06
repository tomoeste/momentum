import { useState, useEffect } from 'react'
import {
  claimSetupToken,
  getSimpleFINStatus,
  disconnectSimpleFIN,
  getAccounts,
  setDebtTerms,
  type Account,
  type SimpleFINStatusResponse,
} from '../lib/tauri-commands'

interface SettingsModalProps {
  isOpen: boolean
  onClose: () => void
}

type SettingsTab = 'simplefin' | 'debt-terms' | 'about'

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

  useEffect(() => {
    if (isOpen) {
      loadSimplefinStatus()
      loadAccounts()
    }
  }, [isOpen])

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
        <div className="flex border-b border-gray-700 px-6">
          <button
            onClick={() => setTab('simplefin')}
            className={`px-4 py-2 border-b-2 font-medium transition-colors ${
              tab === 'simplefin'
                ? 'border-blue-500 text-blue-400'
                : 'border-transparent text-gray-400 hover:text-white'
            }`}
          >
            SimpleFIN
          </button>
          <button
            onClick={() => setTab('debt-terms')}
            className={`px-4 py-2 border-b-2 font-medium transition-colors ${
              tab === 'debt-terms'
                ? 'border-blue-500 text-blue-400'
                : 'border-transparent text-gray-400 hover:text-white'
            }`}
          >
            Debt Terms
          </button>
          <button
            onClick={() => setTab('about')}
            className={`px-4 py-2 border-b-2 font-medium transition-colors ${
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

          {/* About Tab */}
          {tab === 'about' && (
            <div className="space-y-4">
              <h3 className="text-lg font-semibold text-white">About Momentum</h3>
              <div className="space-y-2 text-gray-400 text-sm">
                <p>
                  <strong className="text-white">Version:</strong> 0.0.4
                </p>
                <p>
                  <strong className="text-white">Description:</strong> Local-first budgeting app with cash flow momentum metrics
                </p>
                <p className="pt-2">
                  Momentum uses SimpleFIN to securely sync your financial accounts. Your data is stored locally on your device.
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
