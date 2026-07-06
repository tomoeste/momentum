import { useState } from 'react'
import { Account, TransactionMappingSuggestion } from '../lib/tauri-commands'

interface AccountMappingModalProps {
  isOpen: boolean
  unmappedTransactions: TransactionMappingSuggestion[]
  availableAccounts: Account[]
  onSubmit: (mappings: [string, string][]) => Promise<void>
  onCancel: () => void
}

export function AccountMappingModal({
  isOpen,
  unmappedTransactions,
  availableAccounts,
  onSubmit,
  onCancel,
}: AccountMappingModalProps) {
  const [mappings, setMappings] = useState<Map<string, string>>(new Map())
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [error, setError] = useState<string | null>(null)

  if (!isOpen) return null

  const handleAccountChange = (transactionId: string, accountId: string) => {
    setMappings(prev => {
      const updated = new Map(prev)
      updated.set(transactionId, accountId)
      return updated
    })
  }

  const handleSubmit = async () => {
    setIsSubmitting(true)
    setError(null)

    try {
      const mappingEntries = Array.from(mappings.entries())
      await onSubmit(mappingEntries as [string, string][])
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to submit mappings')
    } finally {
      setIsSubmitting(false)
    }
  }

  const isMappingComplete = mappings.size === unmappedTransactions.length && mappings.size > 0

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow-lg max-w-4xl w-full mx-4 max-h-[90vh] flex flex-col">
        {/* Header */}
        <div className="border-b border-gray-200 dark:border-gray-700 px-6 py-4">
          <h2 className="text-xl font-semibold text-gray-900 dark:text-white">
            Map Transactions to Accounts
          </h2>
          <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
            You have {unmappedTransactions.length} transaction(s) that need to be mapped to the correct account(s).
          </p>
        </div>

        {/* Error message */}
        {error && (
          <div className="bg-red-50 dark:bg-red-900 border border-red-200 dark:border-red-700 text-red-700 dark:text-red-100 px-6 py-3 mx-6 mt-4 rounded">
            {error}
          </div>
        )}

        {/* Content */}
        <div className="flex-1 overflow-auto px-6 py-4">
          <div className="space-y-4">
            {unmappedTransactions.map(txn => (
              <div
                key={txn.transaction_id}
                className="border border-gray-200 dark:border-gray-700 rounded-lg p-4 hover:bg-gray-50 dark:hover:bg-gray-700 transition"
              >
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                  {/* Transaction details */}
                  <div>
                    <p className="text-sm text-gray-600 dark:text-gray-400">Merchant</p>
                    <p className="font-medium text-gray-900 dark:text-white">
                      {txn.merchant || txn.description}
                    </p>
                    <p className="text-sm text-gray-600 dark:text-gray-400 mt-2">
                      {new Date(txn.posted_date).toLocaleDateString()} • {txn.amount > 0 ? '+' : ''} ${Math.abs(txn.amount).toFixed(2)}
                    </p>
                  </div>

                  {/* Account selector */}
                  <div>
                    <label className="block text-sm text-gray-600 dark:text-gray-400 mb-2">
                      Assign to Account
                    </label>
                    <select
                      value={mappings.get(txn.transaction_id) || ''}
                      onChange={e => handleAccountChange(txn.transaction_id, e.target.value)}
                      className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 rounded-md text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                    >
                      <option value="">-- Select account --</option>
                      {availableAccounts.map(account => (
                        <option key={account.id} value={account.id}>
                          {account.name}
                        </option>
                      ))}
                    </select>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Footer */}
        <div className="border-t border-gray-200 dark:border-gray-700 px-6 py-4 flex justify-end gap-3">
          <button
            onClick={onCancel}
            disabled={isSubmitting}
            className="px-4 py-2 text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-700 rounded-md hover:bg-gray-200 dark:hover:bg-gray-600 disabled:opacity-50 transition"
          >
            Cancel
          </button>
          <button
            onClick={handleSubmit}
            disabled={!isMappingComplete || isSubmitting}
            className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition"
          >
            {isSubmitting ? 'Submitting...' : 'Submit Mappings'}
          </button>
        </div>
      </div>
    </div>
  )
}
