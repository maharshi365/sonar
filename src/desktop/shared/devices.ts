export interface AudioInputDevice {
  id: string
  name: string
  isDefault: boolean
}

export interface ComputeDevice {
  id: string
  index: number
  name: string
  kind: string
  memory: number
}
