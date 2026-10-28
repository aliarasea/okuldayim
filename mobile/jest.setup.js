// Testlerde ikon fontları yüklenmez (jest ortamında asset kaydı yoktur); ikonlar boş çizilir.
jest.mock('@expo/vector-icons', () => {
  const Ionicons = () => null;
  Ionicons.glyphMap = {};
  return { Ionicons };
});
