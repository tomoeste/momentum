import { toast } from 'sonner'

export function showErrorToast(message: string, description?: string) {
  toast.error(message, {
    description,
    duration: 5000,
  })
}

export function showSuccessToast(message: string, description?: string) {
  toast.success(message, {
    description,
    duration: 3000,
  })
}

export function showInfoToast(message: string, description?: string) {
  toast.info(message, {
    description,
    duration: 3000,
  })
}

export function showLoadingToast(message: string) {
  return toast.loading(message)
}
