import type { ThaiIDData } from '../types'
import './CardInfo.css'

interface Props {
  data: ThaiIDData
}

const MTH = ['', 'ม.ค.', 'ก.พ.', 'มี.ค.', 'เม.ย.', 'พ.ค.', 'มิ.ย.', 'ก.ค.', 'ส.ค.', 'ก.ย.', 'ต.ค.', 'พ.ย.', 'ธ.ค.']

function dTh(r: string) {
  if (r.length !== 8) return r
  return `${parseInt(r.substring(6, 8))} ${MTH[parseInt(r.substring(4, 6))] || ''} ${r.substring(0, 4)}`
}

function cid(id: string) {
  if (id.length !== 13) return id
  return `${id[0]}-${id.substring(1, 5)}-${id.substring(5, 10)}-${id.substring(10, 12)}-${id[12]}`
}

function sexLabel(s: string) {
  return s === '1' ? 'ชาย' : s === '2' ? 'หญิง' : s || '-'
}

export function CardInfo({ data }: Props) {
  const dobTh    = dTh(data.Birthday)
  const issueTh  = dTh(data.Issue)
  const expireTh = dTh(data.Expire)

  const fullNameTh = [data.Th_Prefix, data.Th_Firstname, data.Th_Middlename, data.Th_Lastname]
    .filter(Boolean).join(' ')

  return (
    <div className="id-card">
      {/* Left Panel - Photo & ID */}
      <div className="id-left">
        <div className="id-photo-wrapper">
          {data.PhotoRaw ? (
            <img
              className="id-photo"
              src={`data:image/jpeg;base64,${data.PhotoRaw}`}
              alt=""
            />
          ) : (
            <div className="id-photo-placeholder">
              <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.2">
                <path d="M20 21v-2a4 4 0 00-4-4H8a4 4 0 00-4 4v2"/>
                <circle cx="12" cy="7" r="4"/>
              </svg>
            </div>
          )}
        </div>
        <div className="id-number">
          {cid(data.Citizenid)}
        </div>
      </div>

      {/* Right Panel */}
      <div className="id-right">

        {/* Thai name (combined) */}
        <div className="id-field">
          <label>ชื่อ-นามสกุล (ภาษาไทย)</label>
          <div className="id-value">{fullNameTh || '-'}</div>
        </div>

        {/* English Name */}
        <div className="id-field">
          <label>FIRSTNAME-LASTNAME (ENGLISH)</label>
          <div className="id-value">{data.full_name_en || '-'}</div>
        </div>

        {/* DOB & Sex */}
        <div className="id-row">
          <div className="id-field half">
            <label>วันเกิด</label>
            <div className="id-value">{dobTh || '-'}</div>
          </div>
          <div className="id-field half">
            <label>เพศ</label>
            <div className="id-value">{sexLabel(data.Sex)}</div>
          </div>
        </div>

        {/* Address components */}
        <div className="id-row">
          <div className="id-field half">
            <label>เลขที่</label>
            <div className="id-value">{data.addrHouseNo || '-'}</div>
          </div>
          <div className="id-field half">
            <label>หมู่ที่</label>
            <div className="id-value">{data.addrVillageNo || '-'}</div>
          </div>
        </div>
        <div className="id-row">
          <div className="id-field half">
            <label>ซอย</label>
            <div className="id-value">{data.addrLane || '-'}</div>
          </div>
          <div className="id-field half">
            <label>ถนน</label>
            <div className="id-value">{data.addrRoad || '-'}</div>
          </div>
        </div>
        <div className="id-row">
          <div className="id-field half">
            <label>ตำบล/แขวง</label>
            <div className="id-value">{data.addrTambol || '-'}</div>
          </div>
          <div className="id-field half">
            <label>อำเภอ/เขต</label>
            <div className="id-value">{data.addrAmphur || '-'}</div>
          </div>
        </div>
        <div className="id-row">
          <div className="id-field half">
            <label>จังหวัด</label>
            <div className="id-value">{data.addrProvince || '-'}</div>
          </div>
        </div>

        {/* Issue & Expiry */}
        <div className="id-row">
          <div className="id-field half">
            <label>วันออกบัตร</label>
            <div className="id-value">{issueTh || '-'}</div>
          </div>
          <div className="id-field half">
            <label>วันหมดอายุ</label>
            <div className="id-value">{expireTh || '-'}</div>
          </div>
        </div>

      </div>
    </div>
  )
}
