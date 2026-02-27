export interface ThaiIDData {
  Citizenid: string
  Th_Prefix: string
  Th_Firstname: string
  Th_Middlename: string
  Th_Lastname: string
  full_name_en: string
  En_Prefix: string
  En_Firstname: string
  En_Middlename: string
  En_Lastname: string
  Birthday: string       // YYYYMMDD Buddhist Era
  Sex: string            // "1" = male, other = female
  Issuer: string
  Issue: string
  Expire: string
  Address: string
  addrHouseNo: string
  addrVillageNo: string
  addrRoad: string
  addrLane: string
  addrTambol: string
  addrAmphur: string
  addrProvince: string
  PhotoRaw: string    
}

export interface CardEvent extends ThaiIDData {
  mode: 'readsmartcard' | 'removedsmartcard'
}
