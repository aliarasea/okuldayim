import { Ionicons } from '@expo/vector-icons';
import { Tabs } from 'expo-router';
import { useTranslation } from 'react-i18next';
import type { ColorValue } from 'react-native';

import { useActiveRole } from '@/core/session/sessionStore';
import { OutboxIndicator } from '@/ui/OutboxIndicator';
import { useColors } from '@/ui/theme';

type IconName = keyof typeof Ionicons.glyphMap;

function TabIcon({ name, color, size }: { name: IconName; color: ColorValue; size: number }) {
  return <Ionicons name={name} color={typeof color === 'string' ? color : undefined} size={size} />;
}

/** Sekmeler aktif role göre gösterilir (bölüm 2 yetki matrisi); erişim kontrolü sunucudadır. */
export default function AppTabsLayout() {
  const { t } = useTranslation();
  const colors = useColors();
  const role = useActiveRole();
  const isParent = role === 'VELI';
  const isTeacher = role === 'OGRETMEN';

  return (
    <Tabs
      screenOptions={{
        headerRight: () => <OutboxIndicator />,
        headerStyle: { backgroundColor: colors.surface },
        headerTintColor: colors.text,
        tabBarStyle: { backgroundColor: colors.surface },
        tabBarActiveTintColor: colors.primary,
        tabBarInactiveTintColor: colors.textMuted,
      }}
    >
      <Tabs.Screen
        name="feed"
        options={{
          title: t('tabs.feed'),
          href: isParent ? undefined : null,
          tabBarIcon: ({ color, size }) => <TabIcon name="today-outline" color={color} size={size} />,
        }}
      />
      <Tabs.Screen
        name="meals"
        options={{
          title: t('tabs.meals'),
          href: isTeacher ? undefined : null,
          tabBarIcon: ({ color, size }) => <TabIcon name="restaurant-outline" color={color} size={size} />,
        }}
      />
      <Tabs.Screen
        name="pickup"
        options={{
          title: t('tabs.pickup'),
          href: isParent || isTeacher ? undefined : null,
          tabBarIcon: ({ color, size }) => <TabIcon name="walk-outline" color={color} size={size} />,
        }}
      />
      <Tabs.Screen
        name="profile"
        options={{
          title: t('tabs.profile'),
          headerShown: false,
          tabBarIcon: ({ color, size }) => <TabIcon name="person-circle-outline" color={color} size={size} />,
        }}
      />
    </Tabs>
  );
}
