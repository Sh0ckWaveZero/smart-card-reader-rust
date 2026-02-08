export interface ThaiIDData {
  citizen_id: string
  full_name_th: string
  full_name_en: string
  date_of_birth: string
  gender: string
  card_issuer: string
  issue_date: string
  expire_date: string
  address: string
  photo: string // Base64 encoded
}

export interface CardEvent {
  type: 'CARD_INSERTED'
  data: ThaiIDData
}
