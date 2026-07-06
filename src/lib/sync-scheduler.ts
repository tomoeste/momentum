import { syncSimpleFin, getSettings } from './tauri-commands'

export class SyncScheduler {
  private timerId: NodeJS.Timeout | null = null

  async start() {
    // Get current settings
    const settings = await getSettings()
    const frequency = settings.sync_settings?.sync_frequency || 'manual'

    // Set up scheduler based on frequency
    await this.setupScheduler(frequency)
  }

  private async setupScheduler(frequency: string) {
    // Clear existing timer
    if (this.timerId) {
      clearInterval(this.timerId)
    }

    if (frequency === 'manual') {
      // No automatic syncing
      return
    }

    // Calculate interval in milliseconds
    let intervalMs = 0
    switch (frequency) {
      case 'on-open':
        // Already handled by app startup
        return
      case '12h':
        intervalMs = 12 * 60 * 60 * 1000
        break
      case '24h':
        intervalMs = 24 * 60 * 60 * 1000
        break
      default:
        return
    }

    // Start interval timer
    this.timerId = setInterval(async () => {
      try {
        console.log(`[SyncScheduler] Executing scheduled sync (${frequency})`)
        await syncSimpleFin({ days_back: 90 })
      } catch (err) {
        console.error('[SyncScheduler] Scheduled sync failed:', err)
        // Don't throw; let app continue even if sync fails
      }
    }, intervalMs)

    console.log(`[SyncScheduler] Started ${frequency} scheduler (interval: ${intervalMs}ms)`)
  }

  stop() {
    if (this.timerId) {
      clearInterval(this.timerId)
      this.timerId = null
    }
  }

  updateFrequency(frequency: string) {
    this.setupScheduler(frequency)
  }
}

// Export singleton instance
export const syncScheduler = new SyncScheduler()
