export interface ThaiIDData {
  Citizenid: string
  Th_Prefix: string
  Th_Firstname: string
  Th_Middlename: string
  Th_Lastname: string
  full_name_en: string
  Birthday: string       // YYYYMMDD Buddhist Era
  Sex: string            // "1" = male, other = female
  card_issuer: string
  issue_date: string
  expire_date: string
  Address: string
  addrHouseNo: string
  addrVillageNo: string
  addrTambol: string
  addrAmphur: string
  PhotoRaw: string       // Base64 encoded
}

export interface CardEvent extends ThaiIDData {
  mode: 'readsmartcard' | 'removedsmartcard'
}
