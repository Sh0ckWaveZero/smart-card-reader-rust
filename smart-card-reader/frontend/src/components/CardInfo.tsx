import type { ThaiIDData } from '../types'
import './CardInfo.css'

interface Props {
  data: ThaiIDData
}

const MTH = ['', 'ม.ค.', 'ก.พ.', 'มี.ค.', 'เม.ย.', 'พ.ค.', 'มิ.ย.', 'ก.ค.', 'ส.ค.', 'ก.ย.', 'ต.ค.', 'พ.ย.', 'ธ.ค.']
const MEN = ['', 'Jan.', 'Feb.', 'Mar.', 'Apr.', 'May', 'Jun.', 'Jul.', 'Aug.', 'Sep.', 'Oct.', 'Nov.', 'Dec.']

function dTh(r: string) {
  if (r.length !== 8) return r
  return `${parseInt(r.substring(6, 8))} ${MTH[parseInt(r.substring(4, 6))] || ''} ${r.substring(0, 4)}`
}
function dEn(r: string) {
  if (r.length !== 8) return r
  return `${parseInt(r.substring(6, 8))} ${MEN[parseInt(r.substring(4, 6))] || ''} ${parseInt(r.substring(0, 4)) - 543}`
}
function cid(id: string) {
  if (id.length !== 13) return id
  return `${id[0]}-${id.substring(1, 5)}-${id.substring(5, 10)}-${id.substring(10, 12)}-${id[12]}`
}

export function CardInfo({ data }: Props) {
  const dobTh = dTh(data.date_of_birth)
  const issueTh = dTh(data.issue_date)
  const expireTh = dTh(data.expire_date)

  return (
    <div className="id-card">
      {/* Left Panel - Photo & ID */}
      <div className="id-left">
        <div className="id-photo-wrapper">
          {data.photo ? (
            <img 
              className="id-photo" 
              src={`data:image/jpeg;base64,${data.photo}`} 
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
          {cid(data.citizen_id)}
        </div>
      </div>

      {/* Right Panel - Information Fields */}
      <div className="id-right">
        {/* Thai Name */}
        <div className="id-field">
          <label>ชื่อ-นามสกุล (ภาษาไทย)</label>
          <div className="id-value">{data.full_name_th || '-'}</div>
        </div>

        {/* English Name */}
        <div className="id-field">
          <label>NAME-SURNAME (ENGLISH)</label>
          <div className="id-value">{data.full_name_en || '-'}</div>
        </div>

        {/* DOB & Gender Row */}
        <div className="id-row">
          <div className="id-field half">
            <label>วันเกิด</label>
            <div className="id-value">{dobTh || '-'}</div>
          </div>
          <div className="id-field half">
            <label>เพศ</label>
            <div className="id-value">{data.gender === '1' ? 'ชาย' : data.gender === '2' ? 'หญิง' : '-'}</div>
          </div>
        </div>

        {/* Issue & Expiry Row */}
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

        {/* Address */}
        <div className="id-field">
          <label>ที่อยู่</label>
          <div className="id-value">{data.address || '-'}</div>
        </div>

        {/* Issuer */}
        <div className="id-field">
          <label>หน่วยงานผู้ออกบัตร</label>
          <div className="id-value">{data.card_issuer || '-'}</div>
        </div>
      </div>
    </div>
  )
}
