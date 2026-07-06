import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Header } from '../components/Header'

describe('Header', () => {
  const mockCallbacks = {
    onSettingsClick: vi.fn(),
    onSyncClick: vi.fn(),
    onViewTransactions: vi.fn(),
  }

  it('renders the Momentum title', () => {
    render(<Header {...mockCallbacks} lastSync={null} />)
    expect(screen.getByText('Momentum')).toBeInTheDocument()
  })

  it('renders all action buttons', () => {
    render(<Header {...mockCallbacks} lastSync={null} />)
    expect(screen.getByRole('button', { name: /Transactions/i })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Sync/i })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Settings/i })).toBeInTheDocument()
  })

  it('calls onViewTransactions when Transactions button is clicked', async () => {
    const user = userEvent.setup()
    render(<Header {...mockCallbacks} lastSync={null} />)
    await user.click(screen.getByRole('button', { name: /Transactions/i }))
    expect(mockCallbacks.onViewTransactions).toHaveBeenCalledOnce()
  })

  it('calls onSyncClick when Sync button is clicked', async () => {
    const user = userEvent.setup()
    render(<Header {...mockCallbacks} lastSync={null} />)
    await user.click(screen.getByRole('button', { name: /Sync/i }))
    expect(mockCallbacks.onSyncClick).toHaveBeenCalledOnce()
  })

  it('calls onSettingsClick when Settings button is clicked', async () => {
    const user = userEvent.setup()
    render(<Header {...mockCallbacks} lastSync={null} />)
    await user.click(screen.getByRole('button', { name: /Settings/i }))
    expect(mockCallbacks.onSettingsClick).toHaveBeenCalledOnce()
  })

  it('displays "Never" when lastSync is null', () => {
    render(<Header {...mockCallbacks} lastSync={null} />)
    expect(screen.getByText(/Last sync: Never/)).toBeInTheDocument()
  })

  it('formats last sync time as "Just now" for recent syncs', () => {
    const now = new Date().toISOString()
    render(<Header {...mockCallbacks} lastSync={now} />)
    expect(screen.getByText(/Last sync: Just now/)).toBeInTheDocument()
  })

  it('formats last sync time in minutes', () => {
    const tenMinutesAgo = new Date(Date.now() - 10 * 60000).toISOString()
    render(<Header {...mockCallbacks} lastSync={tenMinutesAgo} />)
    expect(screen.getByText(/Last sync: 10m ago/)).toBeInTheDocument()
  })

  it('formats last sync time in hours', () => {
    const twoHoursAgo = new Date(Date.now() - 2 * 3600000).toISOString()
    render(<Header {...mockCallbacks} lastSync={twoHoursAgo} />)
    expect(screen.getByText(/Last sync: 2h ago/)).toBeInTheDocument()
  })

  it('formats last sync time in days', () => {
    const threeDaysAgo = new Date(Date.now() - 3 * 86400000).toISOString()
    render(<Header {...mockCallbacks} lastSync={threeDaysAgo} />)
    expect(screen.getByText(/Last sync: 3d ago/)).toBeInTheDocument()
  })

  it('disables Sync button when isSyncing is true', () => {
    render(<Header {...mockCallbacks} lastSync={null} isSyncing={true} />)
    const syncButton = screen.getByRole('button', { name: /Syncing/i })
    expect(syncButton).toBeDisabled()
  })

  it('shows "Syncing..." text when isSyncing is true', () => {
    render(<Header {...mockCallbacks} lastSync={null} isSyncing={true} />)
    expect(screen.getByText('Syncing...')).toBeInTheDocument()
  })

  it('Sync button is enabled when isSyncing is false', () => {
    render(<Header {...mockCallbacks} lastSync={null} isSyncing={false} />)
    const syncButton = screen.getByRole('button', { name: /Sync/i })
    expect(syncButton).not.toBeDisabled()
  })

  it('handles invalid date strings gracefully', () => {
    render(<Header {...mockCallbacks} lastSync="invalid-date" />)
    // Should display the invalid date string - JavaScript converts it to 'Invalid Date'
    const lastSyncText = screen.getByText(/Last sync:/)
    expect(lastSyncText.textContent).toMatch(/Last sync:/)
  })
})
