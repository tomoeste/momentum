import { useState, useEffect } from 'react'
import {
  getTransactions,
  getAccounts,
  recategorizeTransaction,
  type RawTransaction,
  type Account,
} from '../lib/tauri-commands'

interface TransactionListProps {
  onClose: () => void
}

interface TransactionWithDetails extends RawTransaction {
  account_name?: string
  category?: string
  secondary_category?: string
  confidence?: number
  is_manual?: boolean
}

interface DetailModalState {
  isOpen: boolean
  transaction: TransactionWithDetails | null
  category: string
  secondaryCategory: string
  note: string
}

export function TransactionList({ onClose }: TransactionListProps) {
  const [transactions, setTransactions] = useState<TransactionWithDetails[]>([])
  const [accounts, setAccounts] = useState<Account[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [message, setMessage] = useState<string | null>(null)

  // Filter state
  const [searchText, setSearchText] = useState('')
  const [selectedAccount, setSelectedAccount] = useState<string>('all')
  const [selectedCategory, setSelectedCategory] = useState<string>('all')
  const [sortBy, setSortBy] = useState<'date-desc' | 'date-asc' | 'amount-desc' | 'amount-asc'>('date-desc')

  // Pagination state
  const [limit, setLimit] = useState(50)
  const [offset, setOffset] = useState(0)

  // Detail modal state
  const [detailModal, setDetailModal] = useState<DetailModalState>({
    isOpen: false,
    transaction: null,
    category: '',
    secondaryCategory: '',
    note: '',
  })

  const CATEGORIES = [
    'Income',
    'Groceries',
    'Dining Out',
    'Transportation',
    'Utilities',
    'Home & Property',
    'Subscriptions',
    'Shopping',
    'Healthcare',
    'Personal Care',
    'Entertainment',
    'Transfers',
    'Interest',
    'Debt Payments',
    'Uncategorized',
  ]

  useEffect(() => {
    loadData()
  }, [limit, offset, selectedAccount, selectedCategory])

  async function loadData() {
    try {
      setLoading(true)
      setError(null)

      const [txns, accts] = await Promise.all([
        getTransactions({
          account_id: selectedAccount !== 'all' ? selectedAccount : undefined,
          category: selectedCategory !== 'all' ? selectedCategory : undefined,
          limit,
          offset,
        }),
        getAccounts(),
      ])

      setAccounts(accts)

      // Enhance transactions with account names
      const enhanced = txns.map((txn) => ({
        ...txn,
        account_name: accts.find((a) => a.id === txn.account_id)?.name || 'Unknown',
      }))

      // Apply sorting
      enhanced.sort((a, b) => {
        if (sortBy === 'date-desc') {
          return new Date(b.posted_date).getTime() - new Date(a.posted_date).getTime()
        } else if (sortBy === 'date-asc') {
          return new Date(a.posted_date).getTime() - new Date(b.posted_date).getTime()
        } else if (sortBy === 'amount-desc') {
          return b.amount - a.amount
        } else if (sortBy === 'amount-asc') {
          return a.amount - b.amount
        }
        return 0
      })

      // Apply search filter
      if (searchText.trim()) {
        const lowerSearch = searchText.toLowerCase()
        enhanced.filter(
          (txn) =>
            (txn.merchant?.toLowerCase().includes(lowerSearch) ?? false) ||
            txn.description.toLowerCase().includes(lowerSearch) ||
            (txn.account_name?.toLowerCase().includes(lowerSearch) ?? false)
        )
      }

      setTransactions(enhanced)
    } catch (err) {
      console.error('Failed to load transactions:', err)
      setError(err instanceof Error ? err.message : 'Failed to load transactions')
    } finally {
      setLoading(false)
    }
  }

  async function handleOpenDetail(txn: TransactionWithDetails) {
    setDetailModal({
      isOpen: true,
      transaction: txn,
      category: txn.category || 'Uncategorized',
      secondaryCategory: txn.secondary_category || '',
      note: '',
    })
  }

  async function handleSaveCategory() {
    if (!detailModal.transaction) return

    try {
      setError(null)
      setMessage(null)

      await recategorizeTransaction({
        transaction_id: detailModal.transaction.id,
        category: detailModal.category,
        secondary_category: detailModal.secondaryCategory || undefined,
        note: detailModal.note || undefined,
      })

      setMessage('Transaction recategorized successfully')
      setDetailModal({ isOpen: false, transaction: null, category: '', secondaryCategory: '', note: '' })

      // Reload transactions to show updated category
      await loadData()
    } catch (err) {
      console.error('Failed to save category:', err)
      setError(err instanceof Error ? err.message : 'Failed to save category')
    }
  }

  function formatCurrency(value: number): string {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    }).format(value)
  }

  function formatDate(date: string): string {
    return new Date(date).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
    })
  }

  return (
    <div className="min-h-screen bg-gray-900 text-white flex flex-col">
      {/* Header */}
      <div className="border-b border-gray-700 px-6 py-4 flex items-center justify-between">
        <h1 className="text-2xl font-bold">Transactions</h1>
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

      <main className="flex-1 overflow-auto p-6">
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

        {/* Filters */}
        <div className="bg-gray-800 rounded-lg p-4 mb-6 border border-gray-700">
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            {/* Search */}
            <div>
              <label className="block text-sm text-gray-400 mb-1">Search</label>
              <input
                type="text"
                placeholder="Merchant or description"
                value={searchText}
                onChange={(e) => {
                  setSearchText(e.target.value)
                  setOffset(0)
                }}
                className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white placeholder-gray-500 focus:outline-none focus:border-blue-500"
              />
            </div>

            {/* Account filter */}
            <div>
              <label className="block text-sm text-gray-400 mb-1">Account</label>
              <select
                value={selectedAccount}
                onChange={(e) => {
                  setSelectedAccount(e.target.value)
                  setOffset(0)
                }}
                className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white focus:outline-none focus:border-blue-500"
              >
                <option value="all">All Accounts</option>
                {accounts.map((acc) => (
                  <option key={acc.id} value={acc.id}>
                    {acc.name}
                  </option>
                ))}
              </select>
            </div>

            {/* Category filter */}
            <div>
              <label className="block text-sm text-gray-400 mb-1">Category</label>
              <select
                value={selectedCategory}
                onChange={(e) => {
                  setSelectedCategory(e.target.value)
                  setOffset(0)
                }}
                className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white focus:outline-none focus:border-blue-500"
              >
                <option value="all">All Categories</option>
                {CATEGORIES.map((cat) => (
                  <option key={cat} value={cat}>
                    {cat}
                  </option>
                ))}
              </select>
            </div>

            {/* Sort */}
            <div>
              <label className="block text-sm text-gray-400 mb-1">Sort</label>
              <select
                value={sortBy}
                onChange={(e) => setSortBy(e.target.value as any)}
                className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white focus:outline-none focus:border-blue-500"
              >
                <option value="date-desc">Date (Newest)</option>
                <option value="date-asc">Date (Oldest)</option>
                <option value="amount-desc">Amount (High to Low)</option>
                <option value="amount-asc">Amount (Low to High)</option>
              </select>
            </div>
          </div>
        </div>

        {/* Transactions Table */}
        {loading ? (
          <div className="text-center py-8 text-gray-400">Loading transactions...</div>
        ) : transactions.length === 0 ? (
          <div className="text-center py-8 text-gray-400">No transactions found</div>
        ) : (
          <div className="bg-gray-800 rounded-lg border border-gray-700 overflow-hidden">
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-gray-700 bg-gray-750">
                    <th className="px-4 py-3 text-left">Date</th>
                    <th className="px-4 py-3 text-left">Merchant</th>
                    <th className="px-4 py-3 text-left">Account</th>
                    <th className="px-4 py-3 text-left">Category</th>
                    <th className="px-4 py-3 text-right">Amount</th>
                    <th className="px-4 py-3 text-center">Action</th>
                  </tr>
                </thead>
                <tbody>
                  {transactions.map((txn) => (
                    <tr key={txn.id} className="border-b border-gray-700 hover:bg-gray-700/50 transition">
                      <td className="px-4 py-3 text-gray-300">{formatDate(txn.posted_date)}</td>
                      <td className="px-4 py-3">
                        <div>
                          <p className="font-medium text-white">{txn.merchant || txn.description}</p>
                          {txn.merchant && <p className="text-xs text-gray-400">{txn.description}</p>}
                        </div>
                      </td>
                      <td className="px-4 py-3 text-gray-300">{txn.account_name}</td>
                      <td className="px-4 py-3">
                        <span className="px-2 py-1 rounded text-xs bg-blue-900 text-blue-200">
                          {txn.category || 'Uncategorized'}
                        </span>
                      </td>
                      <td className="px-4 py-3 text-right font-medium">
                        <span className={txn.amount >= 0 ? 'text-green-400' : 'text-red-400'}>
                          {formatCurrency(txn.amount)}
                        </span>
                      </td>
                      <td className="px-4 py-3 text-center">
                        <button
                          onClick={() => handleOpenDetail(txn)}
                          className="text-blue-400 hover:text-blue-300 transition-colors"
                        >
                          Edit
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>

            {/* Pagination */}
            <div className="border-t border-gray-700 px-4 py-3 flex items-center justify-between bg-gray-750">
              <div className="flex items-center gap-2">
                <select
                  value={limit}
                  onChange={(e) => {
                    setLimit(parseInt(e.target.value))
                    setOffset(0)
                  }}
                  className="px-3 py-1 bg-gray-700 border border-gray-600 rounded text-white text-sm focus:outline-none focus:border-blue-500"
                >
                  <option value="25">25 per page</option>
                  <option value="50">50 per page</option>
                  <option value="100">100 per page</option>
                </select>
              </div>

              <div className="flex items-center gap-2">
                <span className="text-sm text-gray-400">
                  Showing {offset + 1}-{Math.min(offset + limit, transactions.length)} of {transactions.length}
                </span>
                <button
                  onClick={() => setOffset(Math.max(0, offset - limit))}
                  disabled={offset === 0}
                  className="px-3 py-1 bg-gray-700 hover:bg-gray-600 disabled:opacity-50 text-white rounded text-sm transition-colors"
                >
                  ← Prev
                </button>
                <button
                  onClick={() => setOffset(offset + limit)}
                  disabled={offset + limit >= transactions.length}
                  className="px-3 py-1 bg-gray-700 hover:bg-gray-600 disabled:opacity-50 text-white rounded text-sm transition-colors"
                >
                  Next →
                </button>
              </div>
            </div>
          </div>
        )}
      </main>

      {/* Detail Modal */}
      {detailModal.isOpen && detailModal.transaction && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-gray-800 rounded-lg shadow-xl max-w-md w-full mx-4 border border-gray-700">
            {/* Header */}
            <div className="flex items-center justify-between px-6 py-4 border-b border-gray-700">
              <h2 className="text-xl font-bold text-white">Edit Category</h2>
              <button
                onClick={() => setDetailModal({ isOpen: false, transaction: null, category: '', secondaryCategory: '', note: '' })}
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

            {/* Content */}
            <div className="px-6 py-4 space-y-4 max-h-[60vh] overflow-y-auto">
              {/* Transaction details */}
              <div className="bg-gray-700 rounded p-3 space-y-2">
                <p className="text-sm text-gray-400">
                  <strong className="text-white">{detailModal.transaction.merchant || detailModal.transaction.description}</strong>
                </p>
                <p className="text-sm text-gray-400">
                  {formatDate(detailModal.transaction.posted_date)} • {formatCurrency(detailModal.transaction.amount)}
                </p>
              </div>

              {/* Category */}
              <div>
                <label className="block text-sm text-gray-400 mb-1">Primary Category</label>
                <select
                  value={detailModal.category}
                  onChange={(e) => setDetailModal((prev) => ({ ...prev, category: e.target.value }))}
                  className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white focus:outline-none focus:border-blue-500"
                >
                  {CATEGORIES.map((cat) => (
                    <option key={cat} value={cat}>
                      {cat}
                    </option>
                  ))}
                </select>
              </div>

              {/* Secondary Category */}
              <div>
                <label className="block text-sm text-gray-400 mb-1">Secondary Category (optional)</label>
                <input
                  type="text"
                  placeholder="e.g., Supermarket, Coffee, Gas"
                  value={detailModal.secondaryCategory}
                  onChange={(e) => setDetailModal((prev) => ({ ...prev, secondaryCategory: e.target.value }))}
                  className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white placeholder-gray-500 focus:outline-none focus:border-blue-500"
                />
              </div>

              {/* Note */}
              <div>
                <label className="block text-sm text-gray-400 mb-1">Note (optional)</label>
                <textarea
                  placeholder="Add a note about this transaction"
                  value={detailModal.note}
                  onChange={(e) => setDetailModal((prev) => ({ ...prev, note: e.target.value }))}
                  rows={2}
                  className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white placeholder-gray-500 focus:outline-none focus:border-blue-500 resize-none"
                />
              </div>
            </div>

            {/* Footer */}
            <div className="border-t border-gray-700 px-6 py-3 bg-gray-750 flex gap-2 justify-end">
              <button
                onClick={() => setDetailModal({ isOpen: false, transaction: null, category: '', secondaryCategory: '', note: '' })}
                className="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded font-medium transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={handleSaveCategory}
                className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded font-medium transition-colors"
              >
                Save
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
