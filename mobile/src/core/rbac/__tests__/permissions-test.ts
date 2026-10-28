import { can, grantFor } from '../permissions';

describe('yetki matrisi (gereksinim bölüm 2)', () => {
  it('öğün ve aktivite girişini yalnızca yönetici ve kendi sınıfındaki öğretmen yapar', () => {
    expect(grantFor('KURUM_YONETICISI', 'daily.record')).toBe('FULL');
    expect(grantFor('OGRETMEN', 'daily.record')).toBe('OWN');
    expect(can('VELI', 'daily.record')).toBe(false);
    expect(can('YETKILI_TESLIM_ALAN', 'daily.record')).toBe(false);
  });

  it('teslim bildirimini veli ve yetkili kişi gönderir, onayı öğretmen verir', () => {
    expect(can('VELI', 'pickup.notify')).toBe(true);
    expect(can('YETKILI_TESLIM_ALAN', 'pickup.notify')).toBe(true);
    expect(can('OGRETMEN', 'pickup.notify')).toBe(false);
    expect(can('OGRETMEN', 'pickup.confirm')).toBe(true);
    expect(can('VELI', 'pickup.confirm')).toBe(false);
  });

  it('öğretmen sağlık bilgisini düzenleyemez, yalnızca okur', () => {
    expect(grantFor('OGRETMEN', 'health.view')).toBe('READ');
    expect(can('OGRETMEN', 'health.edit')).toBe(false);
  });

  it('rızayı yalnızca veli yönetir; yönetici yalnızca görür', () => {
    expect(grantFor('VELI', 'consent.manage')).toBe('OWN');
    expect(grantFor('KURUM_YONETICISI', 'consent.manage')).toBe('READ');
    expect(can('OGRETMEN', 'consent.manage')).toBe(false);
  });

  it('rol yoksa hiçbir işlem yapılamaz', () => {
    expect(can(null, 'feed.view')).toBe(false);
  });
});
