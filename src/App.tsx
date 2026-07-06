import { useEffect } from 'react'
import './App.css'

function App() {
  useEffect(() => {
    // TODO: Initialize app state, load from database
  }, [])

  return (
    <div className="min-h-screen bg-gray-900 text-white">
      <header className="border-b border-gray-800 p-4">
        <div className="flex justify-between items-center">
          <h1 className="text-2xl font-bold">Momentum</h1>
          <div className="text-sm text-gray-400">
            Last sync: <span id="last-sync">Never</span>
          </div>
        </div>
      </header>

      <main className="p-6">
        <section className="mb-8">
          <div className="flex gap-4 mb-6">
            <button className="px-4 py-2 rounded bg-blue-600 hover:bg-blue-700">
              This Week
            </button>
            <button className="px-4 py-2 rounded bg-gray-800 hover:bg-gray-700">
              This Month
            </button>
          </div>

          <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-5 gap-4">
            {/* Metric Cards */}
            <div className="bg-gray-800 rounded-lg p-4">
              <p className="text-gray-400 text-sm mb-2">Income</p>
              <p className="text-2xl font-bold text-green-400">$0</p>
              <div className="mt-2 h-12 bg-gray-700 rounded"></div>
            </div>

            <div className="bg-gray-800 rounded-lg p-4">
              <p className="text-gray-400 text-sm mb-2">Spending</p>
              <p className="text-2xl font-bold text-red-400">$0</p>
              <div className="mt-2 h-12 bg-gray-700 rounded"></div>
            </div>

            <div className="bg-gray-800 rounded-lg p-4">
              <p className="text-gray-400 text-sm mb-2">Debt Paydown</p>
              <p className="text-2xl font-bold text-blue-400">$0</p>
              <div className="mt-2 h-12 bg-gray-700 rounded"></div>
            </div>

            <div className="bg-gray-800 rounded-lg p-4">
              <p className="text-gray-400 text-sm mb-2">Interest Paid</p>
              <p className="text-2xl font-bold text-orange-400">$0</p>
              <div className="mt-2 h-12 bg-gray-700 rounded"></div>
            </div>

            <div className="bg-gray-800 rounded-lg p-4">
              <p className="text-gray-400 text-sm mb-2">Debt Ratio</p>
              <p className="text-2xl font-bold text-yellow-400">0.0x</p>
              <div className="mt-2 h-12 bg-gray-700 rounded"></div>
            </div>
          </div>
        </section>

        <section>
          <h2 className="text-xl font-bold mb-4">Interest Bleed</h2>
          <div className="bg-red-900 bg-opacity-20 border border-red-500 rounded-lg p-4">
            <p className="text-lg font-semibold text-red-300">Interest this month: $0 (0% of income)</p>
            <p className="text-gray-400 mt-2">That's $0 per day in overhead.</p>
          </div>
        </section>
      </main>
    </div>
  )
}

export default App
