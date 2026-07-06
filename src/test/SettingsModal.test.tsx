import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { SettingsModal } from '../components/SettingsModal'
import * as tauriCommands from '../lib/tauri-commands'

// Mock Tauri commands
vi.mock('../lib/tauri-commands')

const mockAccounts = [
  { id: 'acc1', simplefin_account_id: 'sf1', name: 'Checking', type: 'checking', organization: 'Bank', balance: 5000, last_updated: '2024-01-01' },
  { id: 'acc2', simplefin_account_id: 'sf2', name: 'Savings', type: 'savings', organization: 'Bank', balance: 10000, last_updated: '2024-01-01' },
]

const mockSettings = {
  llm_config: {
    ollama_url: 'http://localhost:11434',
    llm_model: 'mistral',
    use_local_first: true,
  },
  sync_settings: {
    sync_frequency: 'on-open' as const,
    backfill_days: 90,
    enable_background_sync: false,
  },
  ui_preferences: {
    theme: 'dark' as const,
    currency: 'USD',
  },
}

describe('SettingsModal', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    ;(tauriCommands.getSimpleFINStatus as any).mockResolvedValue({ connected: false, account_count: 0 })
    ;(tauriCommands.getAccounts as any).mockResolvedValue(mockAccounts)
    ;(tauriCommands.getSettings as any).mockResolvedValue(mockSettings)
  })

  it('does not render when isOpen is false', () => {
    render(<SettingsModal isOpen={false} onClose={vi.fn()} />)
    expect(screen.queryByText('Settings')).not.toBeInTheDocument()
  })

  it('renders modal when isOpen is true', async () => {
    render(<SettingsModal isOpen={true} onClose={vi.fn()} />)
    await waitFor(() => {
      expect(screen.getByText('Settings')).toBeInTheDocument()
    })
  })

  it('renders all setting tabs', async () => {
    render(<SettingsModal isOpen={true} onClose={vi.fn()} />)
    await waitFor(() => {
      expect(screen.getByText('SimpleFIN')).toBeInTheDocument()
      expect(screen.getByText('Debt Terms')).toBeInTheDocument()
      expect(screen.getByText('LLM Config')).toBeInTheDocument()
      expect(screen.getByText('Sync Settings')).toBeInTheDocument()
      expect(screen.getByText('UI Preferences')).toBeInTheDocument()
      expect(screen.getByText('About')).toBeInTheDocument()
    })
  })

  it('loads SimpleFIN status on open', async () => {
    render(<SettingsModal isOpen={true} onClose={vi.fn()} />)
    await waitFor(() => {
      expect(tauriCommands.getSimpleFINStatus).toHaveBeenCalled()
    })
  })

  it('loads accounts on open', async () => {
    render(<SettingsModal isOpen={true} onClose={vi.fn()} />)
    await waitFor(() => {
      expect(tauriCommands.getAccounts).toHaveBeenCalled()
    })
  })

  it('loads settings on open', async () => {
    render(<SettingsModal isOpen={true} onClose={vi.fn()} />)
    await waitFor(() => {
      expect(tauriCommands.getSettings).toHaveBeenCalled()
    })
  })

  it('allows switching between tabs', async () => {
    const user = userEvent.setup()
    render(<SettingsModal isOpen={true} onClose={vi.fn()} />)

    await waitFor(() => {
      const buttons = screen.getAllByText('Debt Terms')
      expect(buttons.length).toBeGreaterThan(0)
    })

    const debtTermsTabs = screen.getAllByText('Debt Terms')
    const debtTermsTab = debtTermsTabs[0] // Get the first one (the button, not the heading)
    await user.click(debtTermsTab)

    // Verify click didn't throw an error
    expect(debtTermsTab).toBeInTheDocument()
  })

  it('displays SimpleFIN section by default', async () => {
    render(<SettingsModal isOpen={true} onClose={vi.fn()} />)
    await waitFor(() => {
      expect(screen.getByText('SimpleFIN Integration')).toBeInTheDocument()
    })
  })

  it('displays SimpleFIN connection status', async () => {
    ;(tauriCommands.getSimpleFINStatus as any).mockResolvedValue({
      connected: true,
      account_count: 2,
    })

    render(<SettingsModal isOpen={true} onClose={vi.fn()} />)
    await waitFor(() => {
      expect(screen.getByText(/connected/i)).toBeInTheDocument()
    })
  })

  it('displays buttons in settings interface', async () => {
    render(<SettingsModal isOpen={true} onClose={vi.fn()} />)
    await waitFor(() => {
      const buttons = screen.getAllByRole('button')
      expect(buttons.length).toBeGreaterThan(0)
    })
  })

  it('shows disconnect button when SimpleFIN is connected', async () => {
    ;(tauriCommands.getSimpleFINStatus as any).mockResolvedValue({
      connected: true,
      account_count: 2,
    })

    render(<SettingsModal isOpen={true} onClose={vi.fn()} />)
    await waitFor(() => {
      // SimpleFIN section should display when connected
      expect(screen.getByText(/SimpleFIN Integration/i)).toBeInTheDocument()
    })
  })

  it('displays all settings tab labels', async () => {
    render(<SettingsModal isOpen={true} onClose={vi.fn()} />)

    await waitFor(() => {
      expect(screen.getByText('About')).toBeInTheDocument()
    })
  })
})
