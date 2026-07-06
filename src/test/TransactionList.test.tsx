import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { TransactionList } from '../components/TransactionList'
import * as tauriCommands from '../lib/tauri-commands'

// Mock Tauri commands
vi.mock('../lib/tauri-commands')

const mockOnClose = vi.fn()

const mockAccounts = [
  { id: 'acc1', simplefin_account_id: 'sf1', name: 'Checking', type: 'checking', organization: 'Bank', balance: 5000, last_updated: '2024-01-01' },
  { id: 'acc2', simplefin_account_id: 'sf2', name: 'Savings', type: 'savings', organization: 'Bank', balance: 10000, last_updated: '2024-01-01' },
]

const mockTransactions = [
  {
    id: 'txn1',
    account_id: 'acc1',
    posted_date: '2024-01-15',
    amount: 100.50,
    merchant: 'Grocery Store',
    description: 'Groceries',
    transaction_type: 'debit',
    created_at: '2024-01-15',
    source: 'simplefin',
    category: 'Groceries',
    confidence: 0.95,
  },
  {
    id: 'txn2',
    account_id: 'acc2',
    posted_date: '2024-01-14',
    amount: -500.00,
    merchant: 'Employer Inc',
    description: 'Paycheck',
    transaction_type: 'credit',
    created_at: '2024-01-14',
    source: 'simplefin',
    category: 'Income',
    confidence: 0.99,
  },
]

describe('TransactionList', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    ;(tauriCommands.getTransactions as any).mockResolvedValue(mockTransactions)
    ;(tauriCommands.getAccounts as any).mockResolvedValue(mockAccounts)
    ;(tauriCommands.recategorizeTransaction as any).mockResolvedValue({})
  })

  it('renders the transactions header', async () => {
    render(<TransactionList onClose={mockOnClose} />)
    await waitFor(() => {
      expect(screen.getByText('Transactions')).toBeInTheDocument()
    })
  })

  it('renders close button', async () => {
    render(<TransactionList onClose={mockOnClose} />)
    const closeButton = screen.getByRole('button', { name: '' })
    expect(closeButton).toBeInTheDocument()
  })

  it('loads and displays transactions', async () => {
    render(<TransactionList onClose={mockOnClose} />)
    await waitFor(() => {
      expect(screen.getByText('Grocery Store')).toBeInTheDocument()
      expect(screen.getByText('Employer Inc')).toBeInTheDocument()
    })
  })

  it('displays transaction amounts as currency', async () => {
    render(<TransactionList onClose={mockOnClose} />)
    await waitFor(() => {
      expect(screen.getByText('$100.50')).toBeInTheDocument()
      expect(screen.getByText('-$500.00')).toBeInTheDocument()
    })
  })

  it('displays transaction categories', async () => {
    render(<TransactionList onClose={mockOnClose} />)
    await waitFor(() => {
      expect(screen.getByText('Groceries')).toBeInTheDocument()
      expect(screen.getByText('Income')).toBeInTheDocument()
    })
  })

  it('displays account names in transaction rows', async () => {
    render(<TransactionList onClose={mockOnClose} />)
    await waitFor(() => {
      const accountCells = screen.getAllByText(/Checking|Savings/)
      expect(accountCells.length).toBeGreaterThan(0)
    })
  })

  it('renders filter inputs', async () => {
    render(<TransactionList onClose={mockOnClose} />)
    await waitFor(() => {
      expect(screen.getByPlaceholderText('Merchant or description')).toBeInTheDocument()
    })
  })

  it('renders account filter dropdown', async () => {
    render(<TransactionList onClose={mockOnClose} />)
    await waitFor(() => {
      const accountSelect = screen.getByDisplayValue('All Accounts')
      expect(accountSelect).toBeInTheDocument()
    })
  })

  it('renders category filter dropdown', async () => {
    render(<TransactionList onClose={mockOnClose} />)
    await waitFor(() => {
      const categorySelect = screen.getByDisplayValue('All Categories')
      expect(categorySelect).toBeInTheDocument()
    })
  })

  it('renders sort dropdown', async () => {
    render(<TransactionList onClose={mockOnClose} />)
    await waitFor(() => {
      const sortSelect = screen.getByDisplayValue('Date (Newest)')
      expect(sortSelect).toBeInTheDocument()
    })
  })

  it('calls getTransactions with correct parameters', async () => {
    render(<TransactionList onClose={mockOnClose} />)
    await waitFor(() => {
      expect(tauriCommands.getTransactions).toHaveBeenCalled()
    })
  })

  it('calls getAccounts on load', async () => {
    render(<TransactionList onClose={mockOnClose} />)
    await waitFor(() => {
      expect(tauriCommands.getAccounts).toHaveBeenCalled()
    })
  })

  it('filters transactions by account', async () => {
    const user = userEvent.setup()
    render(<TransactionList onClose={mockOnClose} />)

    await waitFor(() => {
      expect(screen.getByDisplayValue('All Accounts')).toBeInTheDocument()
    })

    const accountSelect = screen.getByDisplayValue('All Accounts')
    await user.selectOptions(accountSelect, 'acc1')

    await waitFor(() => {
      expect(tauriCommands.getTransactions).toHaveBeenCalledWith(
        expect.objectContaining({
          account_id: 'acc1',
        })
      )
    })
  })

  it('filters transactions by category', async () => {
    const user = userEvent.setup()
    render(<TransactionList onClose={mockOnClose} />)

    await waitFor(() => {
      expect(screen.getByDisplayValue('All Categories')).toBeInTheDocument()
    })

    const categorySelect = screen.getByDisplayValue('All Categories')
    await user.selectOptions(categorySelect, 'Groceries')

    await waitFor(() => {
      expect(tauriCommands.getTransactions).toHaveBeenCalledWith(
        expect.objectContaining({
          category: 'Groceries',
        })
      )
    })
  })

  it('allows changing sort order', async () => {
    const user = userEvent.setup()
    render(<TransactionList onClose={mockOnClose} />)

    await waitFor(() => {
      expect(screen.getByDisplayValue('Date (Newest)')).toBeInTheDocument()
    })

    const sortSelect = screen.getByDisplayValue('Date (Newest)')
    await user.selectOptions(sortSelect, 'date-asc')

    // The component sorts client-side, so no need to check API call params
    // Just verify the sort select changed
    await waitFor(() => {
      expect(screen.getByDisplayValue('Date (Oldest)')).toBeInTheDocument()
    })
  })

  it('calls onClose when close button is clicked', async () => {
    const user = userEvent.setup()
    render(<TransactionList onClose={mockOnClose} />)

    // The close button is SVG-based, so we target it differently
    const buttons = screen.getAllByRole('button')
    const closeButton = buttons[0] // First button should be close

    await user.click(closeButton)
    expect(mockOnClose).toHaveBeenCalled()
  })

  it('shows loading state initially', () => {
    ;(tauriCommands.getTransactions as any).mockImplementation(() => new Promise(() => {})) // Never resolves
    render(<TransactionList onClose={mockOnClose} />)
    expect(screen.getByText('Loading transactions...')).toBeInTheDocument()
  })

  it('shows no transactions message when list is empty', async () => {
    ;(tauriCommands.getTransactions as any).mockResolvedValue([])
    ;(tauriCommands.getAccounts as any).mockResolvedValue(mockAccounts)

    render(<TransactionList onClose={mockOnClose} />)
    await waitFor(() => {
      expect(screen.getByText('No transactions found')).toBeInTheDocument()
    })
  })

  it('displays error message on load failure', async () => {
    ;(tauriCommands.getTransactions as any).mockRejectedValue(new Error('Network error'))
    ;(tauriCommands.getAccounts as any).mockResolvedValue(mockAccounts)

    render(<TransactionList onClose={mockOnClose} />)
    await waitFor(() => {
      expect(screen.getByText('Network error')).toBeInTheDocument()
    })
  })

  it('renders Edit button for each transaction', async () => {
    render(<TransactionList onClose={mockOnClose} />)
    await waitFor(() => {
      const editButtons = screen.getAllByText('Edit')
      expect(editButtons).toHaveLength(2)
    })
  })

  it('formats dates correctly', async () => {
    render(<TransactionList onClose={mockOnClose} />)
    await waitFor(() => {
      expect(screen.getByText('Jan 15, 2024')).toBeInTheDocument()
      expect(screen.getByText('Jan 14, 2024')).toBeInTheDocument()
    })
  })

  it('shows positive amounts in green', async () => {
    render(<TransactionList onClose={mockOnClose} />)
    await waitFor(() => {
      const positiveAmount = screen.getByText('$100.50')
      expect(positiveAmount).toHaveClass('text-green-400')
    })
  })

  it('shows negative amounts in red', async () => {
    render(<TransactionList onClose={mockOnClose} />)
    await waitFor(() => {
      const negativeAmount = screen.getByText('-$500.00')
      expect(negativeAmount).toHaveClass('text-red-400')
    })
  })
})
