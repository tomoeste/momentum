import { useState, useEffect } from 'react'
import './App.css'
import { getDashboardMetrics, getOpportunityScenarios, syncSimpleFin, Period, DashboardMetrics, GetOpportunityScenariosResponse } from './lib/tauri-commands'
import { Header } from './components/Header'
import { SettingsModal } from './components/SettingsModal'

function formatCurrency(value: number): string {
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: 'USD',
    minimumFractionDigits: 0,
    maximumFractionDigits: 2,
  }).format(value)
}

function App() {
  const [period, setPeriod] = useState<Period>('month')
  const [metrics, setMetrics] = useState<DashboardMetrics | null>(null)
  const [scenarios, setScenarios] = useState<GetOpportunityScenariosResponse | null>(null)
  const [loading, setLoading] = useState(true)
  const [syncing, setSyncing] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [settingsOpen, setSettingsOpen] = useState(false)

  useEffect(() => {
    loadData()
  }, [period])

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
      // Reload data after sync
      await loadData()
    } catch (err) {
      console.error('Failed to sync:', err)
      setError(err instanceof Error ? err.message : 'Sync failed')
    } finally {
      setSyncing(false)
    }
  }

  return (
    <div className="min-h-screen bg-gray-900 text-white flex flex-col">
      <Header
        lastSync={metrics?.last_sync || null}
        onSettingsClick={() => setSettingsOpen(true)}
        onSyncClick={handleSync}
        isSyncing={syncing}
      />

      <main className="p-6 flex-1">
        {error && (
          <div className="mb-4 p-4 bg-red-900 bg-opacity-20 border border-red-500 rounded-lg text-red-300">
            {error}
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
    </div>
  )
}

export default App
