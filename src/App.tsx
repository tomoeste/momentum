import { useState, useEffect } from 'react'
import { Toaster } from 'sonner'
import './App.css'
import { getDashboardMetrics, getOpportunityScenarios, syncSimpleFin, shouldSyncOnOpen, getTransactionMappingSuggestions, submitTransactionMappings, Period, DashboardMetrics, GetOpportunityScenariosResponse, GetTransactionMappingSuggestionsResponse } from './lib/tauri-commands'
import { Header } from './components/Header'
import { SettingsModal } from './components/SettingsModal'
import { AccountMappingModal } from './components/AccountMappingModal'
import { TransactionList } from './components/TransactionList'
import { syncScheduler } from './lib/sync-scheduler'
import { showErrorToast, showSuccessToast } from './lib/toast-utils'

type AppView = 'dashboard' | 'transactions'

function formatCurrency(value: number): string {
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: 'USD',
    minimumFractionDigits: 0,
    maximumFractionDigits: 2,
  }).format(value)
}

function App() {
  const [view, setView] = useState<AppView>('dashboard')
  const [period, setPeriod] = useState<Period>('month')
  const [metrics, setMetrics] = useState<DashboardMetrics | null>(null)
  const [scenarios, setScenarios] = useState<GetOpportunityScenariosResponse | null>(null)
  const [loading, setLoading] = useState(true)
  const [syncing, setSyncing] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [settingsOpen, setSettingsOpen] = useState(false)
  const [mappingModalOpen, setMappingModalOpen] = useState(false)
  const [mappingSuggestions, setMappingSuggestions] = useState<GetTransactionMappingSuggestionsResponse | null>(null)

  // Check if sync should run on app open (>24h since last sync)
  useEffect(() => {
    checkAndSync()

    // Start sync scheduler for frequency-based syncing
    syncScheduler.start()

    // Cleanup on unmount
    return () => {
      syncScheduler.stop()
    }
  }, [])

  // Load data when period changes
  useEffect(() => {
    loadData()
  }, [period])

  async function checkAndSync() {
    try {
      const shouldSync = await shouldSyncOnOpen()
      if (shouldSync) {
        console.log('Auto-syncing: more than 24 hours since last sync')
        setSyncing(true)
        try {
          await syncSimpleFin({ days_back: 90 })
          await checkForMappingNeeded()
          await loadData()
          showSuccessToast('Auto-sync complete', 'Your accounts and transactions are up to date')
        } catch (err) {
          console.error('Auto-sync failed:', err)
          const errorMessage = err instanceof Error ? err.message : 'Auto-sync failed'
          setError(errorMessage)
          showErrorToast('Auto-sync failed', errorMessage)
        } finally {
          setSyncing(false)
        }
      } else {
        // Just load initial data if no sync needed
        await loadData()
      }
    } catch (err) {
      console.error('Failed to check sync status:', err)
      // Still load data even if check fails
      await loadData()
    }
  }

  async function checkForMappingNeeded() {
    try {
      const suggestions = await getTransactionMappingSuggestions()
      if (suggestions.mapping_required && suggestions.unmapped_transactions.length > 0) {
        setMappingSuggestions(suggestions)
        setMappingModalOpen(true)
      }
    } catch (err) {
      console.error('Failed to check mapping requirements:', err)
      // Don't block the user if mapping check fails
    }
  }

  async function loadData() {
    try {
      setLoading(true)
      setError(null)

      const [metricsData, scenariosData] = await Promise.all([
        getDashboardMetrics(period),
        getOpportunityScenarios(),
      ])

      setMetrics(metricsData)
      setScenarios(scenariosData)
    } catch (err) {
      console.error('Failed to load data:', err)
      setError(err instanceof Error ? err.message : 'Unknown error')
    } finally {
      setLoading(false)
    }
  }

  async function handleSync() {
    setSyncing(true)
    setError(null)

    try {
      await syncSimpleFin({ days_back: 90 })
      await checkForMappingNeeded()
      // Reload data after sync
      await loadData()
      showSuccessToast('Sync complete', 'Your accounts and transactions are up to date')
    } catch (err) {
      console.error('Failed to sync:', err)
      const errorMessage = err instanceof Error ? err.message : 'Sync failed'
      setError(errorMessage)
      showErrorToast('Sync failed', errorMessage)
    } finally {
      setSyncing(false)
    }
  }

  async function handleMappingSubmit(mappings: [string, string][]) {
    try {
      await submitTransactionMappings({ mappings })
      setMappingModalOpen(false)
      setMappingSuggestions(null)
      // Reload data after mapping submission
      await loadData()
    } catch (err) {
      console.error('Failed to submit mappings:', err)
      throw err
    }
  }

  function handleMappingCancel() {
    setMappingModalOpen(false)
    setMappingSuggestions(null)
  }

  // Handle view transitions
  if (view === 'transactions') {
    return (
      <div className="min-h-screen bg-gray-900 text-white flex flex-col">
        <TransactionList onClose={() => setView('dashboard')} />
        <SettingsModal isOpen={settingsOpen} onClose={() => setSettingsOpen(false)} />
        <Toaster theme="dark" />
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-gray-900 text-white flex flex-col">
      <Header
        lastSync={metrics?.last_sync || null}
        onSettingsClick={() => setSettingsOpen(true)}
        onSyncClick={handleSync}
        isSyncing={syncing}
        onViewTransactions={() => setView('transactions')}
      />

      <main className="p-6 flex-1">
        {error && (
          <div className="mb-4 p-4 bg-red-900 bg-opacity-20 border border-red-500 rounded-lg text-red-300 flex items-center justify-between">
            <div className="flex-1">
              <p className="font-semibold mb-1">Sync Error</p>
              <p className="text-sm">{error}</p>
            </div>
            <button
              onClick={handleSync}
              disabled={syncing}
              className="ml-4 px-3 py-1 bg-red-600 hover:bg-red-700 disabled:opacity-50 text-white rounded text-sm font-medium whitespace-nowrap transition-colors"
            >
              {syncing ? 'Retrying...' : 'Retry'}
            </button>
          </div>
        )}

        <section className="mb-8">
          <div className="flex gap-4 mb-6">
            <button
              onClick={() => setPeriod('week')}
              className={`px-4 py-2 rounded ${
                period === 'week' ? 'bg-blue-600' : 'bg-gray-800 hover:bg-gray-700'
              }`}
            >
              This Week
            </button>
            <button
              onClick={() => setPeriod('month')}
              className={`px-4 py-2 rounded ${
                period === 'month' ? 'bg-blue-600' : 'bg-gray-800 hover:bg-gray-700'
              }`}
            >
              This Month
            </button>
          </div>

          {loading ? (
            <div className="text-center py-8 text-gray-400">Loading...</div>
          ) : metrics ? (
            <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-5 gap-4">
              <div className="bg-gray-800 rounded-lg p-4">
                <p className="text-gray-400 text-sm mb-2">Income</p>
                <p className="text-2xl font-bold text-green-400">{formatCurrency(metrics.income)}</p>
                <div className="mt-2 h-12 bg-gray-700 rounded"></div>
              </div>

              <div className="bg-gray-800 rounded-lg p-4">
                <p className="text-gray-400 text-sm mb-2">Spending</p>
                <p className="text-2xl font-bold text-red-400">{formatCurrency(metrics.spending)}</p>
                <div className="mt-2 h-12 bg-gray-700 rounded"></div>
              </div>

              <div className="bg-gray-800 rounded-lg p-4">
                <p className="text-gray-400 text-sm mb-2">Debt Paydown</p>
                <p className="text-2xl font-bold text-blue-400">{formatCurrency(metrics.debt_paydown)}</p>
                <div className="mt-2 h-12 bg-gray-700 rounded"></div>
              </div>

              <div className="bg-gray-800 rounded-lg p-4">
                <p className="text-gray-400 text-sm mb-2">Interest Paid</p>
                <p className="text-2xl font-bold text-orange-400">{formatCurrency(metrics.interest_paid)}</p>
                <div className="mt-2 h-12 bg-gray-700 rounded"></div>
              </div>

              <div className="bg-gray-800 rounded-lg p-4">
                <p className="text-gray-400 text-sm mb-2">Debt Ratio</p>
                <p className="text-2xl font-bold text-yellow-400">{metrics.debt_ratio.toFixed(2)}x</p>
                <div className="mt-2 h-12 bg-gray-700 rounded"></div>
              </div>
            </div>
          ) : null}
        </section>

        {metrics && (
          <section className="mb-8">
            <h2 className="text-xl font-bold mb-4">Interest Bleed</h2>
            <div className="bg-red-900 bg-opacity-20 border border-red-500 rounded-lg p-4">
              <p className="text-lg font-semibold text-red-300">
                Interest this month: {formatCurrency(metrics.interest_paid)}
                {metrics.income > 0 && ` (${((metrics.interest_paid / metrics.income) * 100).toFixed(1)}% of income)`}
              </p>
              {metrics.interest_paid > 0 && (
                <p className="text-gray-400 mt-2">
                  That's {formatCurrency(metrics.interest_paid / 30)} per day in overhead.
                </p>
              )}
            </div>
          </section>
        )}

        {scenarios && (
          <section>
            <h2 className="text-xl font-bold mb-4">Opportunity Cost Scenarios</h2>
            <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
              <p className="text-gray-300 mb-4">
                Total debt: {formatCurrency(scenarios.total_debt)} @ {(scenarios.weighted_apr * 100).toFixed(2)}% APR
              </p>
              {scenarios.scenarios.map((scenario, idx) => (
                <div key={idx} className="mb-3 p-3 bg-gray-700 rounded">
                  <p className="font-semibold">
                    Cut {formatCurrency(scenario.monthly_cut)}/month
                  </p>
                  <p className="text-sm text-gray-300 mt-1">
                    Save {scenario.months_saved.toFixed(1)} months • {formatCurrency(scenario.interest_saved)} interest
                  </p>
                </div>
              ))}
            </div>
          </section>
        )}
      </main>

      <SettingsModal isOpen={settingsOpen} onClose={() => setSettingsOpen(false)} />

      {mappingSuggestions && (
        <AccountMappingModal
          isOpen={mappingModalOpen}
          unmappedTransactions={mappingSuggestions.unmapped_transactions}
          availableAccounts={mappingSuggestions.available_accounts}
          onSubmit={handleMappingSubmit}
          onCancel={handleMappingCancel}
        />
      )}

      <Toaster theme="dark" />
    </div>
  )
}

export default App
