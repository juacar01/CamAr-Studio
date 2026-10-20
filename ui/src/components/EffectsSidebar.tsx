import React, { useState, useEffect } from 'react';
import {
  Sparkles,
  Smile,
  Clapperboard,
  Palette,
  SunMedium,
  Image as ImageIcon,
  ChevronRight,
  RotateCcw,
  Maximize2,
  X,
  Droplet,
  Eye,
  Bookmark,
  Sliders,
  Check,
  Film,
  Plus,
  Save,
} from 'lucide-react';
import type { EffectState, EngineTelemetry, MakeupPresetItem, MakeupPresetConfig, BaseAppearanceState, InstalledEffectItem } from '../types';
import { engineClient } from '../services/wsClient';

interface EffectsSidebarProps {
  effects: EffectState;
  telemetry: EngineTelemetry;
  onChange: (updated: EffectState) => void;
  onReset: () => void;
}

// Tonos para el selector de "Base / Tez" (Incluye swatches Bigo 01-04)
const SKIN_TONES = [
  { id: 'none', label: 'Sin base', color: 'transparent', isNone: true },
  { id: '01', label: '01 Encendido', color: '#f5e0d1' },
  { id: '02', label: '02 Piel', color: '#eed1bd' },
  { id: '03', label: '03 Tez', color: '#d9a678' },
  { id: '04', label: '04 Bronceado', color: '#b87a4d' },
  { id: 'tone-1', label: 'Ébano cálido', color: '#824523' },
  { id: 'tone-2', label: 'Bronce rojizo', color: '#9d584b' },
  { id: 'tone-3', label: 'Canela tostado', color: '#b06f47' },
  { id: 'tone-4', label: 'Trigueño dorado', color: '#c78453' },
  { id: 'tone-5', label: 'Neutro rosado', color: '#cf917c' },
  { id: 'tone-6', label: 'Beige claro', color: '#deb08d' },
  { id: 'tone-7', label: 'Porcelana', color: '#ebd0bd' },
];

// 8 Opciones para el selector de "Lentes de contacto / Color de ojos"
const IRIS_COLORS = [
  { id: 'none', label: 'Natural', color: 'transparent', isNone: true },
  { id: 'honey', label: 'Miel', color: '#c68a35' },
  { id: 'hazel', label: 'Avellana', color: '#887d3b' },
  { id: 'emerald', label: 'Esmeralda', color: '#2f9361' },
  { id: 'ice_blue', label: 'Azul Hielo', color: '#68b5e8' },
  { id: 'sapphire', label: 'Zafiro', color: '#275db0' },
  { id: 'gray', label: 'Gris Perla', color: '#9da7b3' },
  { id: 'violet', label: 'Violeta', color: '#9761b0' },
];

const LIPSTICK_COLORS = [
  { id: 'none', label: 'Natural (Sin labial)', color: 'transparent', isNone: true },
  { id: 'rose_natural', label: 'Rosa Terciopelo', color: '#dc4b69' },
  { id: 'coral_peach', label: 'Coral Melocotón', color: '#e86858' },
  { id: 'soft_pink', label: 'Rosa Pastel', color: '#e57399' },
  { id: 'nude_velvet', label: 'Nude Cálido', color: '#c47568' },
  { id: 'classic_red', label: 'Rojo Carmesí', color: '#c9243b' },
  { id: 'berry_plum', label: 'Ciruela Berry', color: '#9e2b5c' },
  { id: 'wine_burgundy', label: 'Vino Borgoña', color: '#7a1c38' },
];

const EYESHADOW_COLORS = [
  { id: 'none', label: 'Sin sombra', color: 'transparent', isNone: true },
  { id: 'bronze', label: 'Bronce Cálido', color: '#b8734d' },
  { id: 'rose', label: 'Rosa Dorado', color: '#d1607a' },
  { id: 'peach', label: 'Melocotón Glow', color: '#e0856b' },
  { id: 'plum', label: 'Ciruela Ahumada', color: '#8c3861' },
  { id: 'smoky_black', label: 'Negro Ahumado', color: '#2e2e33' },
  { id: 'black_tea', label: 'Té Negro Chic', color: '#422822' },
  { id: 'gold', label: 'Oro Metálico', color: '#d9ad52' },
  { id: 'champagne', label: 'Champagne Shimmer', color: '#ebd1b8' },
];

const EYESHADOW_STYLES = [
  { id: 'natural_nude', label: 'Natural Nude', desc: 'Difuminado suave' },
  { id: 'smoky_glam', label: 'Smoky Glam', desc: 'Ahumado con profundidad' },
  { id: 'cat_wing', label: 'Cat Wing', desc: 'Alargado felino hacia sien' },
  { id: 'sunset_glow', label: 'Sunset Glow', desc: 'Halo luminoso central' },
  { id: 'grunge_kohl', label: 'Grunge 360°', desc: 'Ahumado oscuro inferior y superior' },
  { id: 'foxy_glam', label: 'Foxy Runway', desc: 'Doble ala + lagrimal felino' },
  { id: 'glitter_pop', label: 'Glitter Pop', desc: 'Purpurina brillante en párpado' },
];

const EYEBROW_COLORS = [
  { id: 'none', label: 'Sin cejas', color: 'transparent', isNone: true },
  { id: 'natural_black', label: 'Negro Natural', color: '#1a1a1c' },
  { id: 'dark_brown', label: 'Castaño Oscuro', color: '#2c1e18' },
  { id: 'medium_brown', label: 'Castaño Medio', color: '#3d281e' },
  { id: 'taupe', label: 'Taupe Cenizo', color: '#523f36' },
  { id: 'ash_blonde', label: 'Rubio Ceniza', color: '#665143' },
];

const EYEBROW_STYLES = [
  { id: 'natural_fill', label: 'Natural Ombré', desc: 'Relleno suave difuminado' },
  { id: 'defined_arch', label: 'Arco Definido', desc: 'Microblading con trazos nítidos' },
  { id: 'korean_straight', label: 'Recta Coreana', desc: 'Forma horizontal juvenil' },
  { id: 'fluffy_feathered', label: 'Fluffy Pluma', desc: 'Efecto laminado peinado' },
];

const BEARD_COLORS = [
  { id: 'natural', label: 'Tono Natural', color: '#1a1614', desc: 'Oscurece canas y densifica respetando tu pelo' },
  { id: 'black', label: 'Negro Azabache', color: '#0d0d0f', desc: 'Negro profundo estilo barbería' },
  { id: 'dark_brown', label: 'Castaño Oscuro', color: '#2b1c15', desc: 'Castaño cálido y elegante' },
  { id: 'medium_brown', label: 'Castaño Medio', color: '#3d261b', desc: 'Tono café avellana natural' },
];

const DEFAULT_PRESETS: MakeupPresetItem[] = [
  { id: 'black_tea_chic', name: 'Black Tea Chic', category: 'hermoso', avatar: '/presets/black_tea_chic.png', is_favorite: true },
  { id: 'charm', name: 'Charm', category: 'estilo_ins', avatar: 'https://images.unsplash.com/photo-1534528741775-53994a69daeb?w=200&auto=format&fit=crop&q=80', is_favorite: true },
  { id: 'sakura_light', name: 'Sakura light', category: 'estilo_ins', avatar: 'https://images.unsplash.com/photo-1517841905240-472988babdf9?w=200&auto=format&fit=crop&q=80', is_favorite: true },
  { id: 'pink_frosting', name: 'Pink Frosting', category: 'estilo_ins', avatar: 'https://images.unsplash.com/photo-1524504388940-b1c1722653e1?w=200&auto=format&fit=crop&q=80', is_favorite: false },
  { id: 'juice', name: 'juice', category: 'estilo_ins', avatar: 'https://images.unsplash.com/photo-1544005313-94ddf0286df2?w=200&auto=format&fit=crop&q=80', is_favorite: false },
  { id: 'supermodel', name: 'Supermodel', category: 'vogue', avatar: 'https://images.unsplash.com/photo-1494790108377-be9c29b29330?w=200&auto=format&fit=crop&q=80', is_favorite: true },
  { id: 'natural_glow', name: 'Natural Glow', category: 'vogue', avatar: 'https://images.unsplash.com/photo-1539571696357-5a69c17a67c6?w=200&auto=format&fit=crop&q=80', is_favorite: true },
  { id: 'kpop_idol', name: 'K-Pop Idol', category: 'estilo_ins', avatar: 'https://images.unsplash.com/photo-1519085360753-af0119f7cbe7?w=200&auto=format&fit=crop&q=80', is_favorite: false },
  { id: 'masculino_clean', name: 'Masculino Clean', category: 'hombre', avatar: 'https://images.unsplash.com/photo-1507003211169-0a1dd7228f2d?w=200&auto=format&fit=crop&q=80', is_favorite: true },
  { id: 'barba_studio', name: 'Barba & Studio', category: 'hombre', avatar: 'https://images.unsplash.com/photo-1500648767791-00dcc994a43e?w=200&auto=format&fit=crop&q=80', is_favorite: false },
  { id: 'editorial_man', name: 'Editorial Man', category: 'hombre', avatar: 'https://images.unsplash.com/photo-1506794778202-cad84cf45f1d?w=200&auto=format&fit=crop&q=80', is_favorite: false },
];

export interface FilterLutItem {
  id: string;
  name: string;
  category: 'natural' | 'emocional' | 'vivido' | 'onirico' | 'vintage';
  desc: string;
  gradient: string;
}

export const FILTER_LUTS: FilterLutItem[] = [
  // 1. Natural (Tonos limpios, porcelana, calidez de luz diurna)
  { id: 'nat_porcelain', name: 'Porcelana', category: 'natural', desc: 'Piel blanca translúcida, sombras limpias sin tinte amarillo', gradient: 'linear-gradient(135deg, #fff0f5 0%, #e0f2fe 50%, #ffffff 100%)' },
  { id: 'nat_soft_day', name: 'Día Suave', category: 'natural', desc: 'Luz diurna suave y calidez orgánica para streaming diario', gradient: 'linear-gradient(135deg, #fff8e1 0%, #ffe0b2 50%, #e0f2f1 100%)' },
  { id: 'nat_peach_glow', name: 'Melocotón', category: 'natural', desc: 'Resplandor saludable con tonos cálidos y mejillas rosadas', gradient: 'linear-gradient(135deg, #ffe0b2 0%, #ffccbc 50%, #fbe9e7 100%)' },
  { id: 'nat_japanese', name: 'Japonesa', category: 'natural', desc: 'Película aireada de Tokio, negros suaves y tonos pastel', gradient: 'linear-gradient(135deg, #e8f5e9 0%, #c8e6c9 50%, #e1f5fe 100%)' },
  { id: 'nat_ivory_clean', name: 'Marfil', category: 'natural', desc: 'Claridad cristalina, contraste pulido y balance blanco neutro', gradient: 'linear-gradient(135deg, #fafafa 0%, #e0e0e0 50%, #ffffff 100%)' },
  { id: 'nat_morning_dew', name: 'Rocío Fresco', category: 'natural', desc: 'Sombras cian frescas y altas luces doradas de amanecer', gradient: 'linear-gradient(135deg, #e0f7fa 0%, #b2ebf2 50%, #fff9c4 100%)' },

  // 2. Emocional (Atmósferas dramáticas, atardecer, luz de vela)
  { id: 'emo_golden_sunset', name: 'Atardecer Dorado', category: 'emocional', desc: 'Hora dorada cálida, resplandor ámbar cinematográfico', gradient: 'linear-gradient(135deg, #ff6f00 0%, #ffb300 50%, #ffe082 100%)' },
  { id: 'emo_teal_orange', name: 'Teal & Orange', category: 'emocional', desc: 'Estética Hollywood: sombras turquesa y pieles naranja cálido', gradient: 'linear-gradient(135deg, #00838f 0%, #ff8f00 100%)' },
  { id: 'emo_melancholy', name: 'Melancolía', category: 'emocional', desc: 'Tonos crepusculares violeta y lavanda suavemente desaturados', gradient: 'linear-gradient(135deg, #512da8 0%, #7e57c2 50%, #b39ddb 100%)' },
  { id: 'emo_intimate_dusk', name: 'Crepúsculo Íntimo', category: 'emocional', desc: 'Sombras envolventes y tonalidad vino tinto aterciopelado', gradient: 'linear-gradient(135deg, #4a148c 0%, #880e4f 50%, #ad1457 100%)' },
  { id: 'emo_candlelight', name: 'Luz de Vela', category: 'emocional', desc: 'Incandescencia pura ámbar y atmósfera cálida de fogata', gradient: 'linear-gradient(135deg, #e65100 0%, #f57c00 50%, #ffb74d 100%)' },
  { id: 'emo_midnight', name: 'Azul Medianoche', category: 'emocional', desc: 'Sombras azul zafiro profundo con piel contrastada', gradient: 'linear-gradient(135deg, #0d47a1 0%, #1565c0 50%, #1976d2 100%)' },

  // 3. Vívido (Pop, energía, revista editorial y moda)
  { id: 'viv_pop_glam', name: 'Pop Glam', category: 'vivido', desc: 'Alta saturación y definición con impacto visual en vivo', gradient: 'linear-gradient(135deg, #d81b60 0%, #8e24aa 50%, #3949ab 100%)' },
  { id: 'viv_summer', name: 'Verano Dorado', category: 'vivido', desc: 'Cielos luminosos, tonos marinos y energía solar brillante', gradient: 'linear-gradient(135deg, #0288d1 0%, #00acc1 50%, #fdd835 100%)' },
  { id: 'viv_editorial', name: 'Moda Editorial', category: 'vivido', desc: 'Contraste de portada de revista de alta costura', gradient: 'linear-gradient(135deg, #212121 0%, #c2185b 50%, #f48fb1 100%)' },
  { id: 'viv_candy', name: 'Dulce Candy', category: 'vivido', desc: 'Magentas dulces vibrantes y tonos dorados acaramelados', gradient: 'linear-gradient(135deg, #ff4081 0%, #f48fb1 50%, #ffe082 100%)' },
  { id: 'viv_tropical', name: 'Tropical', category: 'vivido', desc: 'Verdes exuberantes y vitalidad luminosa caribeña', gradient: 'linear-gradient(135deg, #00897b 0%, #43a047 50%, #ffb300 100%)' },
  { id: 'viv_neon', name: 'Neón Cyber', category: 'vivido', desc: 'Contraste eléctrico ultra vívido para transmisiones nocturnas', gradient: 'linear-gradient(135deg, #6200ea 0%, #00e5ff 100%)' },

  // 4. Onírico (Fairy tale, cuarzo, halos y resplandor)
  { id: 'oni_fairy_tale', name: 'Cuento de Hadas', category: 'onirico', desc: 'Difusión de cuento con destellos esmeralda y oro suave', gradient: 'linear-gradient(135deg, #a7ffeb 0%, #b9f6ca 50%, #fff59d 100%)' },
  { id: 'oni_champagne', name: 'Champagne', category: 'onirico', desc: 'Velo cremoso dorado, elegancia sutil y piel aterciopelada', gradient: 'linear-gradient(135deg, #fff8e1 0%, #ffe082 50%, #ffecb3 100%)' },
  { id: 'oni_rose_dream', name: 'Cuarzo Rosa', category: 'onirico', desc: 'Aura rosada pastel romántica y etérea', gradient: 'linear-gradient(135deg, #f8bbd0 0%, #f48fb1 50%, #fce4ec 100%)' },
  { id: 'oni_angelic', name: 'Luz Angelical', category: 'onirico', desc: 'Blancos celestiales elevados con halo luminoso suave', gradient: 'linear-gradient(135deg, #ffffff 0%, #e1f5fe 50%, #ede7f6 100%)' },
  { id: 'oni_mist', name: 'Bruma Mística', category: 'onirico', desc: 'Negros mate desvanecidos con atmósfera de ensueño', gradient: 'linear-gradient(135deg, #cfd8dc 0%, #b0bec5 50%, #eceff1 100%)' },
  { id: 'oni_magic_glow', name: 'Brillo Mágico', category: 'onirico', desc: 'Resplandor dorado en medios tonos que ilumina el rostro', gradient: 'linear-gradient(135deg, #ffd54f 0%, #ffca28 50%, #ffe082 100%)' },

  // 5. Vintage (Película analógica clásica de 35mm, 90s, Polaroid)
  { id: 'vin_kodak_portra', name: 'Kodak Portra', category: 'vintage', desc: 'Tono piel icónico de película de 35mm con negros suaves', gradient: 'linear-gradient(135deg, #d7ccc8 0%, #ffcc80 50%, #b0bec5 100%)' },
  { id: 'vin_fuji_velvia', name: 'Fuji Velvia', category: 'vintage', desc: 'Contraste rico analógico con saturación profunda y verdes esmeralda', gradient: 'linear-gradient(135deg, #2e7d32 0%, #c2185b 50%, #0277bd 100%)' },
  { id: 'vin_retro_90s', name: 'Retro 90s', category: 'vintage', desc: 'Nostalgia de videocámara VHS analógica con calidez suave', gradient: 'linear-gradient(135deg, #ffcc80 0%, #ffe082 50%, #a7ffeb 100%)' },
  { id: 'vin_noir_classic', name: 'Noir B&W', category: 'vintage', desc: 'Monocromo clásico argéntico de alto contraste dramático', gradient: 'linear-gradient(135deg, #111111 0%, #666666 50%, #eeeeee 100%)' },
  { id: 'vin_sepia_antique', name: 'Sepia Elegante', category: 'vintage', desc: 'Tonalidad ámbar chocolate del siglo XIX', gradient: 'linear-gradient(135deg, #5d4037 0%, #8d6e63 50%, #d7ccc8 100%)' },
  { id: 'vin_polaroid', name: 'Polaroid 600', category: 'vintage', desc: 'Instantánea vintage con negros desvanecidos y tinte cálido', gradient: 'linear-gradient(135deg, #fff3e0 0%, #ffb74d 50%, #bcaaa4 100%)' },
  { id: 'vin_cyberpunk', name: 'Cyber 80s', category: 'vintage', desc: 'Crominancia ochentera de sombras turquesa y luces fucsia', gradient: 'linear-gradient(135deg, #00e5ff 0%, #d500f9 100%)' },
];

export const DEFAULT_INSTALLED_EFFECTS: InstalledEffectItem[] = [
  {
    id: 'devil_horns',
    name: 'Cuernos Diablo',
    category: 'Halloween 3D',
    thumb: '/effects/devil_horns/thumb.png',
    badge: 'Halloween',
    badgeColor: 'red',
    variables: [
      {
        key: 'effectIntensity',
        label: 'Opacidad del Accesorio',
        min: 0.0,
        max: 1.0,
        step: 0.01,
        default: 1.0,
        unit: '%',
      },
    ],
  },
  {
    id: 'big_mouth_eyes',
    name: 'Boca Cómica',
    category: 'Viral Cómico',
    thumb: '/effects/funny_big_mouth/thumb.png',
    badge: 'Viral Cómico',
    badgeColor: 'amber',
    variables: [
      {
        key: 'mouthScale',
        label: 'Factor Boca (Agrandado)',
        min: 1.0,
        max: 3.5,
        step: 0.1,
        default: 2.2,
        unit: 'x',
      },
      {
        key: 'eyesScale',
        label: 'Factor Ojos (Agrandado)',
        min: 1.0,
        max: 3.0,
        step: 0.1,
        default: 1.7,
        unit: 'x',
      },
      {
        key: 'eyesDistance',
        label: 'Separación Ocular',
        min: 0.6,
        max: 1.6,
        step: 0.05,
        default: 1.0,
        unit: 'x',
      },
      {
        key: 'effectIntensity',
        label: 'Intensidad / Opacidad',
        min: 0.0,
        max: 1.0,
        step: 0.01,
        default: 1.0,
        unit: '%',
      },
    ],
  },
];

export const EffectsSidebar: React.FC<EffectsSidebarProps> = ({
  effects,
  telemetry: _telemetry,
  onChange,
  onReset: _onReset,
}) => {
  // Tabs laterales principales
  const [mainTab, setMainTab] = useState<'mejorar' | 'efectos' | 'ambiente' | 'filtros' | 'luz' | 'fondo'>('mejorar');

  // Subtabs de "Mejorar"
  const [subTab, setSubTab] = useState<'apariencia' | 'maquillaje' | 'cuerpo'>('apariencia');

  // Filtro conceptual dentro de "Apariencia"
  const [appearanceFilter, setAppearanceFilter] = useState<'todos' | 'rostro' | 'labios' | 'ojos'>('todos');

  // Listado dinámico de Efectos / Accesorios instalados (Cargados desde installed_effects.json)
  const [installedEffects, setInstalledEffects] = useState<InstalledEffectItem[]>(DEFAULT_INSTALLED_EFFECTS);

  useEffect(() => {
    fetch('/effects/installed_effects.json')
      .then((res) => (res.ok ? res.json() : null))
      .then((data) => {
        if (Array.isArray(data) && data.length > 0) {
          setInstalledEffects(data);
        }
      })
      .catch(() => {});
  }, []);

  const DEFAULT_BASE_APPEARANCE: BaseAppearanceState = {
    beautyEnabled: true,
    skinSmoothing: 45,
    skinWhitening: 0,
    faceSlim: 0,
    eyeEnlarge: 0,
    eyeShape: 'auto',
    eyeRound: 0,
    eyeBrighten: 0,
    eyeCatchlight: 0,
    eyelashes: 0,
    browLift: 0,
    noseSlim: 0,
    lipPlump: 0,
    foundationTone: 'none',
    foundationIntensity: 40,
    teethWhitening: 0,
    lipstick: 0,
    lipstickColor: 'rose_natural',
    eyeshadow: 0,
    eyeshadowColor: 'bronze',
    eyeshadowStyle: 'natural_nude',
    eyebrows: 0,
    eyebrowColor: 'dark_brown',
    eyebrowStyle: 'natural_fill',
    contour: 0,
    beardEnhancement: 0,
    beardColor: 'natural',
    irisLighting: 0,
    irisColor: 'none',
  };

  const [baseAppearance, setBaseAppearance] = useState<BaseAppearanceState>(() => {
    return effects.baseAppearance || DEFAULT_BASE_APPEARANCE;
  });

  useEffect(() => {
    if (effects.baseAppearance) {
      setBaseAppearance(effects.baseAppearance);
    }
  }, [effects.baseAppearance]);

  // Estado de Presets Dinámicos de Maquillaje y Filtro Facial
  const [presetCatalog, setPresetCatalog] = useState<MakeupPresetItem[]>(DEFAULT_PRESETS);
  const [favoriteIds, setFavoriteIds] = useState<string[]>(() => {
    return effects.favoritePresets || ['charm', 'sakura_light', 'supermodel', 'natural_glow', 'masculino_clean'];
  });

  useEffect(() => {
    if (effects.favoritePresets && effects.favoritePresets.length > 0) {
      setFavoriteIds(effects.favoritePresets);
    }
  }, [effects.favoritePresets]);

  const [makeupCategory, setMakeupCategory] = useState<string>('todos');
  const [filterCategory, setFilterCategory] = useState<string>('todos');
  const [selectedPresetId, setSelectedPresetId] = useState<string>(() => effects.activePresetId || 'none');
  const [activePresetConfig, setActivePresetConfig] = useState<MakeupPresetConfig | null>(null);
  const [filterIntensity, setFilterIntensity] = useState<number>(() => effects.filterIntensity ? Math.round(effects.filterIntensity * 100) : 75);
  const [makeupIntensity, setMakeupIntensity] = useState<number>(() => effects.makeupIntensity ? Math.round(effects.makeupIntensity * 100) : 65);

  // Estado del modal para guardar un nuevo preset en archivo JSON
  const [isSaveModalOpen, setIsSaveModalOpen] = useState(false);
  const [newPresetName, setNewPresetName] = useState('');
  const [newPresetCategory, setNewPresetCategory] = useState('estilo_ins');

  // Cargar catálogo de /presets/index.json al montar y fusionar con customPresets de camar_beauty.json
  useEffect(() => {
    fetch('/presets/index.json')
      .then((res) => {
        if (res.ok) return res.json();
        throw new Error('Not found');
      })
      .then((items: MakeupPresetItem[]) => {
        if (Array.isArray(items) && items.length > 0) {
          setPresetCatalog((prev) => {
            const customItems = prev.filter((p) => !items.some((i) => i.id === p.id));
            return [...customItems, ...items];
          });
        }
      })
      .catch(() => {});
  }, []);

  useEffect(() => {
    if (effects.customPresets && Object.keys(effects.customPresets).length > 0) {
      setPresetCatalog((prev) => {
        const customItems: MakeupPresetItem[] = Object.values(effects.customPresets!).map((cfg) => ({
          id: cfg.id,
          name: cfg.name || cfg.id,
          category: cfg.category || 'estilo_ins',
          avatar: '✨',
          is_favorite: true,
        }));
        const filtered = prev.filter((p) => !customItems.some((c) => c.id === p.id));
        return [...customItems, ...filtered];
      });
    }
  }, [effects.customPresets]);

  const toggleFavorite = (id: string, e?: React.MouseEvent) => {
    e?.stopPropagation();
    setFavoriteIds((prev) => {
      const next = prev.includes(id) ? prev.filter((item) => item !== id) : [...prev, id];
      onChange({
        ...effects,
        favoritePresets: next,
      });
      return next;
    });
  };

  const applyComposite = (
    base: BaseAppearanceState,
    config: MakeupPresetConfig | null,
    fInt: number,
    mInt: number,
    baseToSave?: BaseAppearanceState
  ) => {
    const baseSmoothing = base.skinSmoothing / 100.0;
    const baseSkinWhitening = (base.skinWhitening || 0) / 100.0;
    const baseFaceSlim = base.faceSlim / 100.0;
    const baseEyeEnlarge = base.eyeEnlarge / 100.0;
    const baseEyeShape = base.eyeShape || 'auto';
    const baseEyeRound = (base.eyeRound || 0) / 100.0;
    const baseEyeBrighten = (base.eyeBrighten || 0) / 100.0;
    const baseEyeCatchlight = (base.eyeCatchlight || 0) / 100.0;
    const baseEyelashes = (base.eyelashes || 0) / 100.0;
    const baseBrowLift = base.browLift / 100.0;
    const baseNoseSlim = base.noseSlim / 50.0;
    const baseLipPlump = base.lipPlump / 100.0;
    const baseFoundation = base.foundationTone;
    const baseFoundationIntensity = (base.foundationIntensity !== undefined ? base.foundationIntensity : 40) / 100.0;
    const baseTeeth = base.teethWhitening / 100.0;
    const baseLipstickColor = base.lipstickColor || 'rose_natural';
    const baseLipstick = baseLipstickColor === 'none' ? 0.0 : base.lipstick / 100.0;
    const baseEyeshadow = base.eyeshadow / 100.0;
    const baseEyebrows = (base.eyebrows || 0) / 100.0;
    const baseContour = base.contour / 100.0;
    const baseBeardEnhancement = (base.beardEnhancement ?? base.beard_enhancement ?? 0) / 100.0;
    const baseBeardColor = base.beardColor || base.beard_color || 'natural';
    const baseIrisLighting = (base.irisLighting || 0) / 100.0;
    const baseIrisColor = base.irisColor || 'none';

    if (!config) {
      onChange({
        ...effects,
        beautyEnabled: base.beautyEnabled ?? true,
        skinSmoothing: baseSmoothing,
        skinWhitening: baseSkinWhitening,
        faceSlim: baseFaceSlim,
        eyeEnlarge: baseEyeEnlarge,
        eyeShape: baseEyeShape,
        eyeRound: baseEyeRound,
        eyeBrighten: baseEyeBrighten,
        eyeCatchlight: baseEyeCatchlight,
        eyelashes: baseEyelashes,
        browLift: baseBrowLift,
        noseSlim: baseNoseSlim,
        lipPlump: baseLipPlump,
        foundationTone: baseFoundation,
        foundationIntensity: baseFoundationIntensity,
        teethWhitening: baseTeeth,
        lipstick: baseLipstick,
        lipstickColor: baseLipstickColor,
        eyeshadow: baseEyeshadow,
        eyeshadowColor: base.eyeshadowColor || 'bronze',
        eyeshadowStyle: base.eyeshadowStyle || 'natural_nude',
        eyebrows: baseEyebrows,
        eyebrowColor: base.eyebrowColor || 'dark_brown',
        eyebrowStyle: base.eyebrowStyle || 'natural_fill',
        contour: baseContour,
        beardEnhancement: baseBeardEnhancement,
        beardColor: baseBeardColor,
        irisLighting: baseIrisLighting,
        irisColor: baseIrisColor,
        baseAppearance: baseToSave || base,
        activePresetId: 'none',
      });
      return;
    }

    const fRatio = fInt / 100.0;
    const mRatio = mInt / 100.0;
    const fl = config.face_light;
    const mk = config.makeup;

    // 1. Capa Filtro / Suavizado de Rostro
    // Los parámetros de calibración de cámara (contraste, brillo, exposición, etc.) NO se tocan
    const compositeSkinSmoothing = Math.min(1.0, Math.max(0.0, baseSmoothing + (fl.skin_smoothing * fRatio * 0.45)));
    const compositeFoundationTone = fRatio > 0.08 && fl.foundation_tone && fl.foundation_tone !== 'none'
      ? fl.foundation_tone
      : baseFoundation;

    // 2. Capa Maquillaje (Cosméticos sobrepuestos aditivamente)
    const compositeLipstick = baseLipstickColor === 'none' && base.lipstick === 0
      ? Number(Math.min(1.0, Math.max(0.0, mk.lipstick * mRatio)).toFixed(3))
      : Number(Math.min(1.0, Math.max(0.0, baseLipstick + mk.lipstick * mRatio)).toFixed(3));
    const compositeLipstickColor = (mk.lipstick_color && mk.lipstick_color !== 'none')
      ? mk.lipstick_color
      : (baseLipstickColor === 'none' && mk.lipstick * mRatio > 0.01 ? 'rose_natural' : baseLipstickColor);
    const compositeEyeshadow = Number(Math.min(1.0, Math.max(0.0, baseEyeshadow + mk.eyeshadow * mRatio)).toFixed(3));
    const compositeEyeshadowColor = mk.eyeshadow_color || base.eyeshadowColor || 'bronze';
    const compositeEyeshadowStyle = mk.eyeshadow_style || base.eyeshadowStyle || 'natural_nude';
    const compositeEyebrows = mk.eyebrows !== undefined
      ? Number(Math.min(1.0, Math.max(0.0, baseEyebrows + mk.eyebrows * mRatio)).toFixed(3))
      : baseEyebrows;
    const compositeEyebrowColor = mk.eyebrow_color || base.eyebrowColor || 'dark_brown';
    const compositeEyebrowStyle = mk.eyebrow_style || base.eyebrowStyle || 'natural_fill';
    const compositeContour = Number(Math.min(1.0, Math.max(0.0, baseContour + mk.contour * mRatio)).toFixed(3));
    const compositeBeard = mk.beard_enhancement !== undefined
      ? Number(Math.min(1.0, Math.max(0.0, baseBeardEnhancement + mk.beard_enhancement * mRatio)).toFixed(3))
      : baseBeardEnhancement;
    const compositeBeardColor = mk.beard_color || baseBeardColor;
    const compositeTeeth = Number(Math.min(1.0, Math.max(0.0, baseTeeth + mk.teeth_whitening * mRatio)).toFixed(3));
    const compositeEyeBrighten = Number(Math.min(1.0, Math.max(0.0, baseEyeBrighten + (mk.eye_brighten || 0) * mRatio)).toFixed(3));
    const compositeEyeCatchlight = Number(Math.min(1.0, Math.max(0.0, baseEyeCatchlight + (mk.eye_catchlight || 0) * mRatio)).toFixed(3));
    const compositeEyelashes = Number(Math.min(1.0, Math.max(0.0, baseEyelashes + (mk.eyelashes || 0) * mRatio)).toFixed(3));
    const compositeLipPlump = Number(Math.min(1.0, Math.max(0.0, baseLipPlump + (mk.lip_plump || 0) * mRatio * 0.5)).toFixed(3));
    const compositeEyeEnlarge = Number(Math.min(1.0, Math.max(0.0, baseEyeEnlarge + (mk.eye_enlarge || 0) * mRatio * 0.4)).toFixed(3));
    const compositeEyeRound = Number(Math.min(1.0, Math.max(0.0, baseEyeRound + (mk.eye_round || 0) * mRatio * 0.4)).toFixed(3));
    const compositeIrisLighting = baseIrisColor === 'none' && baseIrisLighting === 0
      ? (mk.iris_lighting !== undefined ? Number(Math.min(1.0, Math.max(0.0, mk.iris_lighting * mRatio)).toFixed(3)) : 0)
      : (mk.iris_lighting !== undefined
          ? Number(Math.min(1.0, Math.max(0.0, baseIrisLighting + mk.iris_lighting * mRatio)).toFixed(3))
          : baseIrisLighting);
    const compositeIrisColor = (mk.iris_color && mk.iris_color !== 'none') ? mk.iris_color : baseIrisColor;

    onChange({
      ...effects,
      beautyEnabled: true,
      skinSmoothing: Number(compositeSkinSmoothing.toFixed(3)),
      skinWhitening: baseSkinWhitening,
      foundationTone: compositeFoundationTone,
      foundationIntensity: baseFoundationIntensity,
      lipstick: compositeLipstick,
      lipstickColor: compositeLipstickColor,
      eyeshadow: compositeEyeshadow,
      eyeshadowColor: compositeEyeshadowColor,
      eyeshadowStyle: compositeEyeshadowStyle,
      eyebrows: compositeEyebrows,
      eyebrowColor: compositeEyebrowColor,
      eyebrowStyle: compositeEyebrowStyle,
      contour: compositeContour,
      beardEnhancement: compositeBeard,
      beardColor: compositeBeardColor,
      teethWhitening: compositeTeeth,
      eyeBrighten: compositeEyeBrighten,
      eyeCatchlight: compositeEyeCatchlight,
      eyelashes: compositeEyelashes,
      lipPlump: compositeLipPlump,
      eyeEnlarge: compositeEyeEnlarge,
      eyeShape: baseEyeShape,
      eyeRound: compositeEyeRound,
      irisLighting: compositeIrisLighting,
      irisColor: compositeIrisColor,
      faceSlim: baseFaceSlim,
      browLift: baseBrowLift,
      noseSlim: baseNoseSlim,
      baseAppearance: baseToSave || base,
      activePresetId: config?.id || 'none',
      makeupIntensity: mInt / 100.0,
      filterIntensity: fInt / 100.0,
    });
  };

  const updateBaseAppearance = (partial: Partial<BaseAppearanceState>) => {
    setBaseAppearance((prev) => {
      const updated = { ...prev, ...partial };
      applyComposite(updated, activePresetConfig, filterIntensity, makeupIntensity, updated);
      return updated;
    });
  };

  const handleSelectPreset = async (item: MakeupPresetItem | { id: 'none'; isNone: true }) => {
    if ('isNone' in item && item.isNone) {
      setSelectedPresetId('none');
      setActivePresetConfig(null);
      applyComposite(baseAppearance, null, 0, 0);
      return;
    }

    setSelectedPresetId(item.id);

    const customConfig = effects.customPresets?.[item.id];
    if (customConfig) {
      setActivePresetConfig(customConfig);
      const fInt = customConfig.default_filter_intensity ?? 75;
      const mInt = customConfig.default_makeup_intensity ?? 65;
      setFilterIntensity(fInt);
      setMakeupIntensity(mInt);
      const updatedBase: BaseAppearanceState = {
        ...baseAppearance,
        ...(customConfig.makeup.eyeshadow_color ? { eyeshadowColor: customConfig.makeup.eyeshadow_color } : {}),
        ...(customConfig.makeup.eyeshadow_style ? { eyeshadowStyle: customConfig.makeup.eyeshadow_style } : {}),
        ...(customConfig.makeup.iris_color && customConfig.makeup.iris_color !== 'none' ? { irisColor: customConfig.makeup.iris_color } : {}),
        ...(customConfig.makeup.lipstick_color && customConfig.makeup.lipstick_color !== 'none' ? { lipstickColor: customConfig.makeup.lipstick_color } : {}),
      };
      setBaseAppearance(updatedBase);
      applyComposite(updatedBase, customConfig, fInt, mInt, updatedBase);
      return;
    }

    try {
      const res = await fetch(`/presets/${item.id}.json`);
      if (res.ok) {
        const config: MakeupPresetConfig = await res.json();
        setActivePresetConfig(config);
        const fInt = config.default_filter_intensity ?? 75;
        const mInt = config.default_makeup_intensity ?? 65;
        setFilterIntensity(fInt);
        setMakeupIntensity(mInt);
        const updatedBase: BaseAppearanceState = {
          ...baseAppearance,
          ...(config.makeup.eyeshadow_color ? { eyeshadowColor: config.makeup.eyeshadow_color } : {}),
          ...(config.makeup.eyeshadow_style ? { eyeshadowStyle: config.makeup.eyeshadow_style } : {}),
          ...(config.makeup.iris_color && config.makeup.iris_color !== 'none' ? { irisColor: config.makeup.iris_color } : {}),
          ...(config.makeup.lipstick_color && config.makeup.lipstick_color !== 'none' ? { lipstickColor: config.makeup.lipstick_color } : {}),
        };
        setBaseAppearance(updatedBase);
        applyComposite(updatedBase, config, fInt, mInt, updatedBase);
        return;
      }
    } catch {}

    console.warn(`No se pudo cargar /presets/${item.id}.json`);
  };

  const handleSaveCustomPresetFile = () => {
    const trimmed = newPresetName.trim();
    if (!trimmed) return;
    const newId = trimmed.toLowerCase().replace(/[^a-z0-9]/g, '_');
    const presetConfig: MakeupPresetConfig = {
      id: newId,
      name: trimmed,
      category: newPresetCategory,
      default_filter_intensity: filterIntensity,
      default_makeup_intensity: makeupIntensity,
      face_light: {
        brightness: 0.05,
        contrast: 1.05,
        saturation: 1.0,
        white_balance: 0.0,
        exposure: 0.0,
        foundation_tone: baseAppearance.foundationTone,
        skin_smoothing: baseAppearance.skinSmoothing / 100.0,
      },
      makeup: {
        lipstick: baseAppearance.lipstick / 100.0,
        eyeshadow: baseAppearance.eyeshadow / 100.0,
        contour: baseAppearance.contour / 100.0,
        teeth_whitening: baseAppearance.teethWhitening / 100.0,
        eye_brighten: (baseAppearance.eyeBrighten || 0) / 100.0,
        eye_catchlight: (baseAppearance.eyeCatchlight || 0) / 100.0,
        eyelashes: (baseAppearance.eyelashes || 0) / 100.0,
        lip_plump: baseAppearance.lipPlump / 100.0,
        eye_enlarge: baseAppearance.eyeEnlarge / 100.0,
        eye_round: (baseAppearance.eyeRound || 0) / 100.0,
        iris_lighting: (baseAppearance.irisLighting || 0) / 100.0,
        iris_color: baseAppearance.irisColor || 'none',
      },
    };
    const presetItem: MakeupPresetItem = {
      id: newId,
      name: trimmed,
      category: newPresetCategory,
      avatar: '✨',
      is_favorite: true,
    };

    engineClient.savePresetFile(presetConfig, presetItem);
    setPresetCatalog((prev) => [presetItem, ...prev.filter((p) => p.id !== newId)]);
    setSelectedPresetId(newId);
    setActivePresetConfig(presetConfig);
    setIsSaveModalOpen(false);
    setNewPresetName('');
  };

  // Estado de los controles de "Cuerpo"
  const [showBodyBanner, setShowBodyBanner] = useState(true);
  const [contornoCorporal, setContornoCorporal] = useState(0);
  const [contornoBrazos, setContornoBrazos] = useState(0);
  const [cuelloHombros, setCuelloHombros] = useState(0); // Bipolar
  const [piernasRango, setPiernasRango] = useState(0);
  const [piernasSolidez, setPiernasSolidez] = useState(0);

  // Restablecer subtab activo
  const handleResetCurrentSubtab = () => {
    if (subTab === 'apariencia') {
      const resetBase: BaseAppearanceState = {
        ...DEFAULT_BASE_APPEARANCE,
        skinSmoothing: 0,
      };
      setBaseAppearance(resetBase);
      applyComposite(resetBase, activePresetConfig, filterIntensity, makeupIntensity, resetBase);
    } else if (subTab === 'maquillaje') {
      setSelectedPresetId('none');
      setActivePresetConfig(null);
      setFilterIntensity(75);
      setMakeupIntensity(65);
      applyComposite(baseAppearance, null, 0, 0);
    } else if (subTab === 'cuerpo') {
      setContornoCorporal(0);
      setContornoBrazos(0);
      setCuelloHombros(0);
      setPiernasRango(0);
      setPiernasSolidez(0);
    }
  };

  return (
    <aside className="w-[360px] border-l border-neutral-800 bg-[#121214] flex flex-col h-[calc(100vh-4rem)] select-none text-white text-xs font-sans">
      {/* 1. Header con Título y Botón Pop-out */}
      <div className="px-5 pt-3.5 pb-1 flex items-center justify-between">
        <h2 className="text-sm font-bold tracking-wide text-neutral-100 capitalize">
          {mainTab}
        </h2>
        <button
          className="text-neutral-400 hover:text-white p-1 rounded transition-colors"
          title="Abrir en ventana flotante"
        >
          <Maximize2 className="w-3.5 h-3.5" />
        </button>
      </div>

      {/* 2. Barra Superior de Tabs Laterales (Scroll Horizontal) */}
      <div className="flex items-center border-b border-neutral-800/80 px-2 overflow-x-auto no-scrollbar scroll-smooth">
        <button
          onClick={() => setMainTab('mejorar')}
          className={`flex items-center space-x-1 px-3 py-2.5 font-medium whitespace-nowrap transition-all border-b-2 ${
            mainTab === 'mejorar'
              ? 'border-white text-white font-semibold'
              : 'border-transparent text-neutral-400 hover:text-neutral-200'
          }`}
        >
          <Sparkles className="w-3.5 h-3.5 text-neutral-200" />
          <span>Mejorar</span>
        </button>

        <button
          onClick={() => setMainTab('efectos')}
          className={`flex items-center space-x-1 px-3 py-2.5 font-medium whitespace-nowrap transition-all border-b-2 ${
            mainTab === 'efectos'
              ? 'border-white text-white font-semibold'
              : 'border-transparent text-neutral-400 hover:text-neutral-200'
          }`}
        >
          <Smile className="w-3.5 h-3.5" />
          <span>Efectos</span>
        </button>

        <button
          onClick={() => setMainTab('ambiente')}
          className={`flex items-center space-x-1 px-3 py-2.5 font-medium whitespace-nowrap transition-all border-b-2 ${
            mainTab === 'ambiente'
              ? 'border-white text-white font-semibold'
              : 'border-transparent text-neutral-400 hover:text-neutral-200'
          }`}
        >
          <Clapperboard className="w-3.5 h-3.5" />
          <span>Ambiente</span>
        </button>

        <button
          onClick={() => setMainTab('filtros')}
          className={`flex items-center space-x-1 px-3 py-2.5 font-medium whitespace-nowrap transition-all border-b-2 ${
            mainTab === 'filtros'
              ? 'border-white text-white font-semibold'
              : 'border-transparent text-neutral-400 hover:text-neutral-200'
          }`}
        >
          <Palette className="w-3.5 h-3.5" />
          <span>Filtros</span>
        </button>

        <button
          onClick={() => setMainTab('luz')}
          className={`flex items-center space-x-1 px-3 py-2.5 font-medium whitespace-nowrap transition-all border-b-2 ${
            mainTab === 'luz'
              ? 'border-white text-white font-semibold'
              : 'border-transparent text-neutral-400 hover:text-neutral-200'
          }`}
        >
          <SunMedium className="w-3.5 h-3.5" />
          <span>Luz</span>
        </button>

        <button
          onClick={() => setMainTab('fondo')}
          className={`flex items-center space-x-1 px-3 py-2.5 font-medium whitespace-nowrap transition-all border-b-2 ${
            mainTab === 'fondo'
              ? 'border-white text-white font-semibold'
              : 'border-transparent text-neutral-400 hover:text-neutral-200'
          }`}
        >
          <ImageIcon className="w-3.5 h-3.5" />
          <span>Fondo</span>
        </button>

        <div className="pl-1 pr-2 text-neutral-500 shrink-0">
          <ChevronRight className="w-3.5 h-3.5" />
        </div>
      </div>

      {/* 3. CONTENIDO SEGÚN TAB ACTIVO */}
      {mainTab === 'mejorar' && (
        <div className="flex-1 flex flex-col overflow-hidden">
          {/* Subtabs: Apariencia | Maquillaje | Cuerpo & Botón Reset */}
          <div className="flex items-center justify-between px-5 py-3 border-b border-neutral-800/40 bg-[#161618]">
            <div className="flex items-center space-x-2">
              <button
                onClick={() => setSubTab('apariencia')}
                className={`px-3 py-1.5 rounded-lg text-xs font-semibold transition-all ${
                  subTab === 'apariencia'
                    ? 'bg-[#2a2a2e] text-white shadow-sm'
                    : 'text-neutral-400 hover:text-neutral-200'
                }`}
              >
                Apariencia
              </button>

              <button
                onClick={() => setSubTab('maquillaje')}
                className={`px-3 py-1.5 rounded-lg text-xs font-semibold transition-all ${
                  subTab === 'maquillaje'
                    ? 'bg-[#2a2a2e] text-white shadow-sm'
                    : 'text-neutral-400 hover:text-neutral-200'
                }`}
              >
                Maquillaje
              </button>

              <button
                onClick={() => setSubTab('cuerpo')}
                className={`px-3 py-1.5 rounded-lg text-xs font-semibold transition-all ${
                  subTab === 'cuerpo'
                    ? 'bg-[#2a2a2e] text-white shadow-sm'
                    : 'text-neutral-400 hover:text-neutral-200'
                }`}
              >
                Cuerpo
              </button>
            </div>

            <button
              onClick={handleResetCurrentSubtab}
              className="text-neutral-400 hover:text-white p-1 rounded-md transition-colors"
              title="Restablecer valores de este subtab"
            >
              <RotateCcw className="w-4 h-4" />
            </button>
          </div>

          {/* Panel de Contenido con Scroll Vertical Suave */}
          <div className="flex-1 overflow-y-auto px-5 py-4 space-y-5 custom-scroll">
            {/* SUBTAB 1: APARIENCIA */}
            {subTab === 'apariencia' && (
              <div className="space-y-4">
                {/* Switch Maestro: Retoque de Belleza & Piel */}
                <div className="flex items-center justify-between p-3 rounded-xl bg-neutral-900/90 border border-neutral-700/60 shadow-sm">
                  <div className="flex items-center space-x-2.5">
                    <div className={`p-1.5 rounded-lg ${baseAppearance.beautyEnabled ? 'bg-pink-500/20 text-pink-400 border border-pink-500/30' : 'bg-neutral-800 text-neutral-400'}`}>
                      <Sparkles className="w-4 h-4" />
                    </div>
                    <div>
                      <div className="text-xs font-semibold text-white">Retoque Facial & Belleza</div>
                      <div className="text-[10px] text-neutral-400">Base de apariencia anatómica persistente</div>
                    </div>
                  </div>
                  <label className="relative inline-flex items-center cursor-pointer">
                    <input
                      type="checkbox"
                      checked={baseAppearance.beautyEnabled}
                      onChange={(e) => updateBaseAppearance({ beautyEnabled: e.target.checked })}
                      className="sr-only peer"
                    />
                    <div className="w-9 h-5 bg-neutral-700 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-neutral-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-pink-500"></div>
                  </label>
                </div>

                {/* Badge informativo de Capa Base + Preset Superpuesto */}
                {selectedPresetId !== 'none' && (
                  <div className="px-3 py-1.5 rounded-xl bg-pink-500/10 border border-pink-500/20 text-[11px] text-pink-300 flex items-center justify-between">
                    <span>Base de apariencia activa</span>
                    <span className="text-[10px] bg-pink-500/20 px-2 py-0.5 rounded text-pink-200 font-mono">
                      + {presetCatalog.find((p) => p.id === selectedPresetId)?.name || selectedPresetId} (Overlay)
                    </span>
                  </div>
                )}

                {/* Contenedor de Controles de Apariencia: habilitado/deshabilitado según el switch */}
                <div className={`space-y-3.5 transition-all duration-200 ${baseAppearance.beautyEnabled ? 'opacity-100' : 'opacity-40 pointer-events-none select-none'}`}>
                  {/* Barra de Acceso Rápido por Categoría Conceptual */}
                  <div className="grid grid-cols-4 gap-1.5 p-1 bg-[#17171b] border border-neutral-800/80 rounded-xl">
                    {[
                      { id: 'todos', label: 'Todo' },
                      { id: 'rostro', label: '✨ Rostro' },
                      { id: 'labios', label: '💋 Boca' },
                      { id: 'ojos', label: '👁️ Ojos' },
                    ].map((tab) => (
                      <button
                        key={tab.id}
                        onClick={() => setAppearanceFilter(tab.id as 'todos' | 'rostro' | 'labios' | 'ojos')}
                        className={`py-1.5 px-2 rounded-lg text-[11px] font-semibold transition-all ${
                          appearanceFilter === tab.id
                            ? 'bg-pink-500/20 text-pink-300 border border-pink-500/40 shadow-sm'
                            : 'text-neutral-400 hover:text-neutral-200 hover:bg-white/5'
                        }`}
                      >
                        {tab.label}
                      </button>
                    ))}
                  </div>

                  {/* ========================================================= */}
                  {/* GRUPO 1: PIEL & ESCULTURA FACIAL                          */}
                  {/* ========================================================= */}
                  {(appearanceFilter === 'todos' || appearanceFilter === 'rostro') && (
                    <div className="p-3.5 rounded-2xl bg-[#17171b]/90 border border-neutral-800/90 space-y-3.5 shadow-inner">
                      <div className="flex items-center justify-between border-b border-neutral-800/80 pb-2">
                        <div className="flex items-center space-x-2">
                          <div className="w-6 h-6 rounded-lg bg-amber-400/15 border border-amber-400/30 flex items-center justify-center">
                            <Sparkles className="w-3.5 h-3.5 text-amber-300" />
                          </div>
                          <div>
                            <h3 className="text-xs font-bold text-neutral-100 tracking-wide">Piel & Escultura Facial</h3>
                            <p className="text-[10px] text-neutral-400">Porcelana, base de maquillaje y facciones</p>
                          </div>
                        </div>
                      </div>

                      {/* 3.1 Suavizar piel */}
                      <div className="space-y-1.5">
                        <div className="flex items-center justify-between">
                          <span className="flex items-center space-x-2 text-neutral-200 font-medium">
                            <Droplet className="w-3.5 h-3.5 text-neutral-300" />
                            <span>Suavizar piel</span>
                          </span>
                          <span className="font-mono text-neutral-300">{baseAppearance.skinSmoothing}</span>
                        </div>
                        <input
                          type="range"
                          min="0"
                          max="100"
                          value={baseAppearance.skinSmoothing}
                          onChange={(e) => updateBaseAppearance({ skinSmoothing: Number(e.target.value) })}
                          className="w-full h-1 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-white"
                        />
                      </div>

                      {/* 3.2 Aclarar piel */}
                      <div className="space-y-1.5">
                        <div className="flex items-center justify-between">
                          <span className="flex items-center space-x-2 text-neutral-200 font-medium">
                            <Sparkles className="w-3.5 h-3.5 text-amber-300" />
                            <span>Aclarar piel (Porcelana)</span>
                          </span>
                          <span className="font-mono text-amber-300">{baseAppearance.skinWhitening || 0}</span>
                        </div>
                        <input
                          type="range"
                          min="0"
                          max="100"
                          value={baseAppearance.skinWhitening || 0}
                          onChange={(e) => updateBaseAppearance({ skinWhitening: Number(e.target.value) })}
                          className="w-full h-1 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-amber-300"
                        />
                      </div>

                      {/* 3.3 Base / Tez */}
                      <div className="space-y-2 pt-1 border-t border-neutral-800/60">
                        <div className="flex items-center justify-between">
                          <span className="flex items-center space-x-2 text-neutral-200 font-medium">
                            <svg className="w-3.5 h-3.5 text-neutral-300" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                              <circle cx="12" cy="12" r="8" />
                              <path d="M12 8v8M8 12h8" />
                            </svg>
                            <span>Base / Tez</span>
                          </span>
                          <span className="text-[11px] text-neutral-400">
                            {SKIN_TONES.find((t) => t.id === baseAppearance.foundationTone)?.label || 'Sin base'}
                          </span>
                        </div>

                        <div className="flex items-center space-x-2.5 overflow-x-auto py-1 no-scrollbar">
                          {SKIN_TONES.map((tone) => (
                            <button
                              key={tone.id}
                              onClick={() => updateBaseAppearance({ foundationTone: tone.id })}
                              className={`w-7 h-7 rounded-full shrink-0 transition-transform relative flex items-center justify-center ${
                                baseAppearance.foundationTone === tone.id ? 'scale-110 ring-2 ring-white ring-offset-2 ring-offset-[#121214]' : 'opacity-85 hover:opacity-100'
                              }`}
                              style={{ backgroundColor: tone.color }}
                              title={tone.label}
                            >
                              {tone.isNone && (
                                <div className="w-full h-full rounded-full border border-neutral-500 flex items-center justify-center text-neutral-400">
                                  <span className="text-base font-light">⊘</span>
                                </div>
                              )}
                            </button>
                          ))}
                        </div>

                        {baseAppearance.foundationTone !== 'none' && (
                          <div className="space-y-1.5 pt-1 pl-0.5">
                            <div className="flex items-center justify-between">
                              <span className="text-neutral-400 text-xs">Intensidad de base</span>
                              <span className="font-mono text-neutral-300">{baseAppearance.foundationIntensity ?? 40}%</span>
                            </div>
                            <input
                              type="range"
                              min="0"
                              max="100"
                              value={baseAppearance.foundationIntensity ?? 40}
                              onChange={(e) => updateBaseAppearance({ foundationIntensity: Number(e.target.value) })}
                              className="w-full h-1 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-white"
                            />
                          </div>
                        )}
                      </div>

                      {/* 3.4 Contorno Facial */}
                      <div className="space-y-1.5 pt-1 border-t border-neutral-800/60">
                        <div className="flex items-center justify-between">
                          <span className="flex items-center space-x-2 text-neutral-200 font-medium">
                            <svg className="w-3.5 h-3.5 text-neutral-300" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                              <path d="m14 12 3 3-9 9H5v-3l9-9Z" />
                              <path d="M19 10l-2-2 2-2 2 2-2 2Z" />
                              <path d="m18 7 3-3" />
                            </svg>
                            <span>Contorno & Rubor</span>
                          </span>
                          <span className="font-mono text-neutral-300">{baseAppearance.contour}</span>
                        </div>
                        <input
                          type="range"
                          min="0"
                          max="100"
                          value={baseAppearance.contour}
                          onChange={(e) => updateBaseAppearance({ contour: Number(e.target.value) })}
                          className="w-full h-1 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-white"
                        />
                      </div>

                      {/* 3.45 Realce y Tinte de Barba */}
                      <div className="space-y-2 pt-2 border-t border-neutral-800/60">
                        <div className="flex items-center justify-between">
                          <span className="flex items-center space-x-2 text-neutral-200 font-medium">
                            <svg className="w-3.5 h-3.5 text-amber-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                              <path d="M4 10c0 5 3 10 8 10s8-5 8-10" />
                              <path d="M8 14c1 2 2.5 3 4 3s3-1 4-3" />
                              <path d="M9 10h.01M15 10h.01" />
                            </svg>
                            <span>Barba & Visagismo</span>
                          </span>
                          <span className="font-mono text-neutral-300">{baseAppearance.beardEnhancement ?? 0}</span>
                        </div>
                        <input
                          type="range"
                          min="0"
                          max="100"
                          value={baseAppearance.beardEnhancement ?? 0}
                          onChange={(e) => updateBaseAppearance({ beardEnhancement: Number(e.target.value) })}
                          className="w-full h-1 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-amber-400"
                        />
                        {(baseAppearance.beardEnhancement ?? 0) > 0 && (
                          <div className="pt-1.5 space-y-1.5">
                            <span className="text-[10px] text-neutral-400 font-medium">Tono de barba y cobertura de canas</span>
                            <div className="grid grid-cols-2 gap-1.5">
                              {BEARD_COLORS.map((bColor) => {
                                const isSelected = (baseAppearance.beardColor || 'natural') === bColor.id;
                                return (
                                  <button
                                    key={bColor.id}
                                    type="button"
                                    onClick={() => updateBaseAppearance({ beardColor: bColor.id })}
                                    className={`flex items-center space-x-2 px-2.5 py-1.5 rounded-lg text-[11px] border transition-all ${
                                      isSelected
                                        ? 'bg-amber-400/15 border-amber-400/60 text-amber-200 shadow-sm'
                                        : 'bg-[#1b1b1f] border-neutral-800 text-neutral-400 hover:text-neutral-200 hover:border-neutral-700'
                                    }`}
                                  >
                                    <div
                                      className="w-3 h-3 rounded-full border border-neutral-600 flex-shrink-0"
                                      style={{ backgroundColor: bColor.color }}
                                    />
                                    <span className="truncate">{bColor.label}</span>
                                  </button>
                                );
                              })}
                            </div>
                          </div>
                        )}
                      </div>

                      {/* 3.5 Forma (V-Line) */}
                      <div className="space-y-1.5">
                        <div className="flex items-center justify-between">
                          <span className="flex items-center space-x-2 text-neutral-200 font-medium">
                            <svg className="w-3.5 h-3.5 text-neutral-300" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                              <path d="M7 10c0 4 2.5 7 5 7s5-3 5-7" />
                              <path d="M9 7h.01M15 7h.01" />
                            </svg>
                            <span>Forma facial (V-Line)</span>
                          </span>
                          <span className="font-mono text-neutral-300">{baseAppearance.faceSlim}</span>
                        </div>
                        <input
                          type="range"
                          min="0"
                          max="100"
                          value={baseAppearance.faceSlim}
                          onChange={(e) => updateBaseAppearance({ faceSlim: Number(e.target.value) })}
                          className="w-full h-1 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-white"
                        />
                      </div>

                      {/* 3.6 Nariz */}
                      <div className="space-y-1.5">
                        <div className="flex items-center justify-between">
                          <span className="flex items-center space-x-2 text-neutral-200 font-medium">
                            <svg className="w-3.5 h-3.5 text-neutral-300" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                              <path d="M10 6v7a2 2 0 0 0 4 0V6" />
                              <path d="M8 17a2 2 0 0 0 2 2h4a2 2 0 0 0 2-2" />
                            </svg>
                            <span>Afinar nariz</span>
                          </span>
                          <span className="font-mono text-neutral-300">{baseAppearance.noseSlim}</span>
                        </div>
                        <input
                          type="range"
                          min="-50"
                          max="50"
                          value={baseAppearance.noseSlim}
                          onChange={(e) => updateBaseAppearance({ noseSlim: Number(e.target.value) })}
                          className="w-full h-1 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-white"
                        />
                      </div>

                      {/* 3.7 Elevar cejas */}
                      <div className="space-y-1.5">
                        <div className="flex items-center justify-between">
                          <span className="flex items-center space-x-2 text-neutral-200 font-medium">
                            <svg className="w-3.5 h-3.5 text-neutral-300" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                              <path d="M4 14c3-3 7-3 10 0" />
                              <path d="M11 9l-2-2m0 0L7 9m2-2v6" />
                            </svg>
                            <span>Elevar cejas</span>
                          </span>
                          <span className="font-mono text-neutral-300">{baseAppearance.browLift}</span>
                        </div>
                        <input
                          type="range"
                          min="0"
                          max="100"
                          value={baseAppearance.browLift}
                          onChange={(e) => updateBaseAppearance({ browLift: Number(e.target.value) })}
                          className="w-full h-1 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-white"
                        />
                      </div>
                    </div>
                  )}
                  {/* ========================================================= */}
                  {/* GRUPO 2: BOCA & SONRISA (Sprite Labial Color + Volumen) */}
                  {/* ========================================================= */}
                  {(appearanceFilter === 'todos' || appearanceFilter === 'labios') && (
                    <div className="p-3.5 rounded-2xl bg-[#17171b]/90 border border-neutral-800/90 space-y-3.5 shadow-inner">
                      <div className="flex items-center justify-between border-b border-neutral-800/80 pb-2">
                        <div className="flex items-center space-x-2">
                          <div className="w-6 h-6 rounded-lg bg-rose-500/15 border border-rose-500/30 flex items-center justify-center">
                            <svg className="w-3.5 h-3.5 text-rose-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                              <path d="M4 12c3-2 5-1 8-1s5-1 8 1c-3 4-5 5-8 5s-5-1-8-5z" />
                            </svg>
                          </div>
                          <div>
                            <h3 className="text-xs font-bold text-neutral-100 tracking-wide">Boca & Sonrisa</h3>
                            <p className="text-[10px] text-neutral-400">Sprite labial deformable, tono y volumen</p>
                          </div>
                        </div>
                        <span className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-rose-500/10 text-rose-300 border border-rose-500/20">
                          {LIPSTICK_COLORS.find((c) => c.id === (baseAppearance.lipstickColor || 'rose_natural'))?.label.split(' ')[0] || 'Rosa'}
                        </span>
                      </div>

                      {/* 2.1 Selector de Color de Labial (Sprite Deformable) */}
                      <div className="space-y-2">
                        <div className="flex items-center justify-between">
                          <span className="flex items-center space-x-2 text-neutral-200 font-medium">
                            <Palette className="w-3.5 h-3.5 text-rose-400" />
                            <span>Tono de Labial (Sprite)</span>
                          </span>
                          <span className="text-[11px] text-neutral-400">
                            {LIPSTICK_COLORS.find((c) => c.id === (baseAppearance.lipstickColor || 'rose_natural'))?.label || 'Rosa Terciopelo'}
                          </span>
                        </div>
                        <div className="flex items-center space-x-2 overflow-x-auto py-1 no-scrollbar">
                          {LIPSTICK_COLORS.map((tone) => {
                            const currentColor = baseAppearance.lipstick === 0 && baseAppearance.lipstickColor === 'none'
                              ? 'none'
                              : (baseAppearance.lipstickColor || 'rose_natural');
                            const isSelected = currentColor === tone.id;
                            if (tone.isNone) {
                              return (
                                <button
                                  key={tone.id}
                                  onClick={() => updateBaseAppearance({ lipstickColor: 'none', lipstick: 0 })}
                                  className={`w-7 h-7 rounded-full border border-neutral-600 shrink-0 flex items-center justify-center transition-all ${
                                    isSelected ? 'ring-2 ring-rose-400 scale-110 bg-neutral-800' : 'opacity-70 hover:opacity-100'
                                  }`}
                                  title="Sin labial (Natural)"
                                >
                                  <span className="text-xs text-neutral-400">⊘</span>
                                </button>
                              );
                            }
                            return (
                              <button
                                key={tone.id}
                                onClick={() =>
                                  updateBaseAppearance({
                                    lipstickColor: tone.id,
                                    lipstick: (baseAppearance.lipstick || 0) === 0 ? 55 : baseAppearance.lipstick,
                                  })
                                }
                                style={{ backgroundColor: tone.color }}
                                className={`w-7 h-7 rounded-full shrink-0 transition-transform ${
                                  isSelected ? 'ring-2 ring-rose-400 scale-110 shadow-lg' : 'opacity-85 hover:opacity-100'
                                }`}
                                title={tone.label}
                              />
                            );
                          })}
                        </div>
                      </div>

                      {/* 2.2 Intensidad de Labial (Alpha) */}
                      <div className="space-y-1.5">
                        <div className="flex items-center justify-between">
                          <span className="flex items-center space-x-2 text-neutral-200 font-medium">
                            <svg className="w-3.5 h-3.5 text-rose-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                              <path d="M10 14h4v7h-4zM10 8h4v6h-4zM11 3h2l1 5h-4z" />
                            </svg>
                            <span>Intensidad de labial (Alpha)</span>
                          </span>
                          <span className="font-mono text-rose-300">{baseAppearance.lipstick}%</span>
                        </div>
                        <input
                          type="range"
                          min="0"
                          max="100"
                          value={baseAppearance.lipstick}
                          onChange={(e) => {
                            const val = Number(e.target.value);
                            updateBaseAppearance({
                              lipstick: val,
                              lipstickColor:
                                val > 0 && (!baseAppearance.lipstickColor || baseAppearance.lipstickColor === 'none')
                                  ? 'rose_natural'
                                  : baseAppearance.lipstickColor,
                            });
                          }}
                          className="w-full h-1 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-rose-400"
                        />
                      </div>

                      {/* 2.3 Volumen de Labios (Plump) */}
                      <div className="space-y-1.5 pt-1 border-t border-neutral-800/60">
                        <div className="flex items-center justify-between">
                          <span className="flex items-center space-x-2 text-neutral-200 font-medium">
                            <svg className="w-3.5 h-3.5 text-neutral-300" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                              <path d="M4 12c3-2 5-1 8-1s5-1 8 1c-3 4-5 5-8 5s-5-1-8-5z" />
                            </svg>
                            <span>Volumen de labios (Plump)</span>
                          </span>
                          <span className="font-mono text-neutral-300">{baseAppearance.lipPlump}</span>
                        </div>
                        <input
                          type="range"
                          min="0"
                          max="100"
                          value={baseAppearance.lipPlump}
                          onChange={(e) => updateBaseAppearance({ lipPlump: Number(e.target.value) })}
                          className="w-full h-1 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-white"
                        />
                      </div>

                      {/* 2.4 Blanqueamiento de Dientes */}
                      <div className="space-y-1.5">
                        <div className="flex items-center justify-between">
                          <span className="flex items-center space-x-2 text-neutral-200 font-medium">
                            <svg className="w-3.5 h-3.5 text-neutral-300" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                              <path d="M7 6c0 5 1 12 3 12s2-4 2-6 0-6 2-6 2 4 2 6 1 6 3 6 3-7 3-12a5 5 0 0 0-15 0z" />
                            </svg>
                            <span>Blanquear dientes</span>
                          </span>
                          <span className="font-mono text-neutral-300">{baseAppearance.teethWhitening}</span>
                        </div>
                        <input
                          type="range"
                          min="0"
                          max="100"
                          value={baseAppearance.teethWhitening}
                          onChange={(e) => updateBaseAppearance({ teethWhitening: Number(e.target.value) })}
                          className="w-full h-1 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-white"
                        />
                      </div>
                    </div>
                  )}
                  {/* ========================================================= */}
                  {/* GRUPO 3: OJOS & MIRADA (Pupilentes, Pestañas, Geometría)  */}
                  {/* ========================================================= */}
                  {(appearanceFilter === 'todos' || appearanceFilter === 'ojos') && (
                    <div className="p-3.5 rounded-2xl bg-[#17171b]/90 border border-neutral-800/90 space-y-3.5 shadow-inner">
                      <div className="flex items-center justify-between border-b border-neutral-800/80 pb-2">
                        <div className="flex items-center space-x-2">
                          <div className="w-6 h-6 rounded-lg bg-[#00e5c9]/15 border border-[#00e5c9]/30 flex items-center justify-center">
                            <Eye className="w-3.5 h-3.5 text-[#00e5c9]" />
                          </div>
                          <div>
                            <h3 className="text-xs font-bold text-neutral-100 tracking-wide">Ojos & Mirada</h3>
                            <p className="text-[10px] text-neutral-400">Pupilentes alpha, pestañas 360° y brillo</p>
                          </div>
                        </div>
                        <span className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-[#00e5c9]/10 text-[#00e5c9] border border-[#00e5c9]/20">
                          {IRIS_COLORS.find((c) => c.id === (baseAppearance.irisColor || 'none'))?.label || 'Natural'}
                        </span>
                      </div>

                      {/* 1.1 Selector de Color de Pupilentes (Sprites) */}
                      <div className="space-y-2">
                        <div className="flex items-center justify-between">
                          <span className="flex items-center space-x-2 text-neutral-200 font-medium">
                            <Palette className="w-3.5 h-3.5 text-[#00e5c9]" />
                            <span>Pupilentes (Color de ojos)</span>
                          </span>
                          <span className="text-[11px] text-neutral-400">
                            {IRIS_COLORS.find((c) => c.id === (baseAppearance.irisColor || 'none'))?.label || 'Sin pupilente'}
                          </span>
                        </div>
                        <div className="flex items-center space-x-2 overflow-x-auto py-1 no-scrollbar">
                          {IRIS_COLORS.map((tone) => {
                            const isSelected = (baseAppearance.irisColor || 'none') === tone.id;
                            if (tone.isNone) {
                              return (
                                <button
                                  key={tone.id}
                                  onClick={() => updateBaseAppearance({ irisColor: 'none', irisLighting: 0 })}
                                  className={`w-7 h-7 rounded-full border border-neutral-600 shrink-0 flex items-center justify-center transition-all ${
                                    isSelected ? 'ring-2 ring-[#00e5c9] scale-110 bg-neutral-800' : 'opacity-70 hover:opacity-100'
                                  }`}
                                  title="Sin pupilente (Natural)"
                                >
                                  <span className="text-xs text-neutral-400">⊘</span>
                                </button>
                              );
                            }
                            return (
                              <button
                                key={tone.id}
                                onClick={() =>
                                  updateBaseAppearance({
                                    irisColor: tone.id,
                                    irisLighting: (baseAppearance.irisLighting || 0) === 0 ? 65 : baseAppearance.irisLighting,
                                  })
                                }
                                className={`w-8 h-8 rounded-full shrink-0 transition-transform bg-[#0d0d10] border border-neutral-700/80 p-0.5 overflow-hidden flex items-center justify-center ${
                                  isSelected ? 'ring-2 ring-[#00e5c9] scale-110 shadow-lg' : 'opacity-85 hover:opacity-100'
                                }`}
                                title={tone.label}
                              >
                                <img
                                  src={`/sprites/lenses/${tone.id}.png`}
                                  alt={tone.label}
                                  className="w-full h-full object-contain pointer-events-none"
                                />
                              </button>
                            );
                          })}
                        </div>
                      </div>

                      {/* 1.2 Intensidad de Pupilente (Alpha) */}
                      <div className="space-y-1.5">
                        <div className="flex items-center justify-between">
                          <span className="flex items-center space-x-2 text-neutral-200 font-medium">
                            <svg className="w-3.5 h-3.5 text-[#00e5c9]" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                              <circle cx="12" cy="12" r="8" />
                              <circle cx="12" cy="12" r="3" />
                              <path d="M12 2v2m0 16v2M2 12h2m16 0h2" />
                            </svg>
                            <span>Intensidad de pupilente (Alpha)</span>
                          </span>
                          <span className="font-mono text-[#00e5c9]">{baseAppearance.irisLighting || 0}%</span>
                        </div>
                        <input
                          type="range"
                          min="0"
                          max="100"
                          value={baseAppearance.irisLighting || 0}
                          onChange={(e) => {
                            const val = Number(e.target.value);
                            updateBaseAppearance({
                              irisLighting: val,
                              irisColor:
                                val > 0 && (!baseAppearance.irisColor || baseAppearance.irisColor === 'none')
                                  ? 'honey'
                                  : baseAppearance.irisColor,
                            });
                          }}
                          className="w-full h-1 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-[#00e5c9]"
                        />
                      </div>

                      {/* 1.3 Pestañas & Delineado 360° */}
                      <div className="space-y-1.5 pt-1 border-t border-neutral-800/60">
                        <div className="flex items-center justify-between">
                          <span className="flex items-center space-x-2 text-neutral-200 font-medium">
                            <Eye className="w-3.5 h-3.5 text-pink-400" />
                            <span>Pestañas & Delineado (Sup + Inf)</span>
                          </span>
                          <span className="font-mono text-pink-300">{baseAppearance.eyelashes || 0}</span>
                        </div>
                        <input
                          type="range"
                          min="0"
                          max="100"
                          value={baseAppearance.eyelashes || 0}
                          onChange={(e) => updateBaseAppearance({ eyelashes: Number(e.target.value) })}
                          className="w-full h-1 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-pink-400"
                        />
                      </div>

                      {/* 1.4 Forma de Ojos (Biométrica IA / Estilo Anatómico) */}
                      <div className="space-y-1.5">
                        <div className="flex items-center justify-between">
                          <span className="flex items-center space-x-2 text-neutral-200 font-medium">
                            <Eye className="w-3.5 h-3.5 text-neutral-300" />
                            <span>Forma de ojos</span>
                          </span>
                          <span className="text-xs text-neutral-400 capitalize">
                            {baseAppearance.eyeShape === 'auto' || !baseAppearance.eyeShape ? 'Auto (IA)' :
                             baseAppearance.eyeShape === 'almond' ? 'Almendrado' :
                             baseAppearance.eyeShape === 'foxy' ? 'Foxy' :
                             baseAppearance.eyeShape === 'asian' ? 'Asiático' :
                             baseAppearance.eyeShape === 'doe' ? 'Muñeca' :
                             baseAppearance.eyeShape === 'hooded' ? 'Encapotado' : baseAppearance.eyeShape}
                          </span>
                        </div>
                        <div className="grid grid-cols-3 gap-1.5 pt-0.5">
                          {[
                            { id: 'auto', label: 'Auto (IA)' },
                            { id: 'almond', label: 'Almendra' },
                            { id: 'foxy', label: 'Foxy' },
                            { id: 'asian', label: 'Asiático' },
                            { id: 'doe', label: 'Muñeca' },
                            { id: 'hooded', label: 'Encapotado' },
                          ].map((opt) => {
                            const active = (baseAppearance.eyeShape || 'auto') === opt.id;
                            return (
                              <button
                                key={opt.id}
                                type="button"
                                onClick={() => updateBaseAppearance({ eyeShape: opt.id })}
                                className={`px-2 py-1.5 text-xs rounded-md font-medium transition-colors border text-center ${
                                  active
                                    ? 'bg-neutral-100 text-neutral-900 border-neutral-100 font-semibold shadow-sm'
                                    : 'bg-[#1e1e22] text-neutral-300 border-[#2b2b30] hover:bg-[#28282e] hover:text-white'
                                }`}
                              >
                                {opt.label}
                              </button>
                            );
                          })}
                        </div>
                      </div>

                      {/* 1.5 Agrandar Ojos (Escala Isotrópica 1:1) */}
                      <div className="space-y-1.5">
                        <div className="flex items-center justify-between">
                          <span className="flex items-center space-x-2 text-neutral-200 font-medium">
                            <Maximize2 className="w-3.5 h-3.5 text-neutral-300" />
                            <span>Agrandar ojos</span>
                          </span>
                          <span className="font-mono text-neutral-300">{baseAppearance.eyeEnlarge}</span>
                        </div>
                        <input
                          type="range"
                          min="0"
                          max="100"
                          value={baseAppearance.eyeEnlarge}
                          onChange={(e) => updateBaseAppearance({ eyeEnlarge: Number(e.target.value) })}
                          className="w-full h-1 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-white"
                        />
                      </div>

                      {/* 1.6 Brillo de Pupila (Catchlight) */}
                      <div className="space-y-1.5">
                        <div className="flex items-center justify-between">
                          <span className="flex items-center space-x-2 text-neutral-200 font-medium">
                            <SunMedium className="w-3.5 h-3.5 text-amber-300" />
                            <span>Brillo de Pupila (Catchlight)</span>
                          </span>
                          <span className="font-mono text-amber-300">{baseAppearance.eyeCatchlight || 0}</span>
                        </div>
                        <input
                          type="range"
                          min="0"
                          max="100"
                          value={baseAppearance.eyeCatchlight || 0}
                          onChange={(e) => updateBaseAppearance({ eyeCatchlight: Number(e.target.value) })}
                          className="w-full h-1 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-amber-300"
                        />
                      </div>

                      {/* 1.7 Claridad (Blanco del ojo) */}
                      <div className="space-y-1.5">
                        <div className="flex items-center justify-between">
                          <span className="flex items-center space-x-2 text-neutral-200 font-medium">
                            <Sparkles className="w-3.5 h-3.5 text-cyan-400" />
                            <span>Claridad (Blanco del ojo)</span>
                          </span>
                          <span className="font-mono text-cyan-300">{baseAppearance.eyeBrighten || 0}</span>
                        </div>
                        <input
                          type="range"
                          min="0"
                          max="100"
                          value={baseAppearance.eyeBrighten || 0}
                          onChange={(e) => updateBaseAppearance({ eyeBrighten: Number(e.target.value) })}
                          className="w-full h-1 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-cyan-400"
                        />
                      </div>

                      {/* 1.8 Sombra de Ojos (Sprite Deformable + Paleta + Estilos) */}
                      <div className="space-y-3 pt-2 border-t border-neutral-800/60">
                        <div className="flex items-center justify-between">
                          <span className="flex items-center space-x-2 text-neutral-200 font-medium">
                            <Palette className="w-3.5 h-3.5 text-amber-400" />
                            <span>Sombra de ojos (Sprite)</span>
                          </span>
                          <span className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-amber-500/10 text-amber-300 border border-amber-500/20">
                            {EYESHADOW_STYLES.find((s) => s.id === (baseAppearance.eyeshadowStyle || 'natural_nude'))?.label || 'Natural'}
                          </span>
                        </div>

                        {/* 1.8.1 Selector de Estilo de Sombra */}
                        <div className="grid grid-cols-2 gap-1.5">
                          {EYESHADOW_STYLES.map((style) => {
                            const isSelected = (baseAppearance.eyeshadowStyle || 'natural_nude') === style.id;
                            return (
                              <button
                                key={style.id}
                                onClick={() =>
                                  updateBaseAppearance({
                                    eyeshadowStyle: style.id,
                                    eyeshadow: (baseAppearance.eyeshadow || 0) === 0 ? 55 : baseAppearance.eyeshadow,
                                    eyeshadowColor: (baseAppearance.eyeshadowColor || 'none') === 'none' ? 'bronze' : baseAppearance.eyeshadowColor,
                                  })
                                }
                                className={`px-2.5 py-1.5 rounded-lg border text-left transition-all ${
                                  isSelected
                                    ? 'bg-amber-500/15 border-amber-500/40 text-amber-200 font-semibold'
                                    : 'bg-[#1e1e22]/60 border-neutral-800 text-neutral-400 hover:text-neutral-200 hover:border-neutral-700'
                                }`}
                              >
                                <div className="text-[11px] font-medium leading-none mb-0.5">{style.label}</div>
                                <div className="text-[9px] text-neutral-500 truncate leading-tight">{style.desc}</div>
                              </button>
                            );
                          })}
                        </div>

                        {/* 1.8.2 Selector de Color de Sombra */}
                        <div className="space-y-1.5">
                          <div className="flex items-center justify-between">
                            <span className="text-[11px] text-neutral-400">Tono de pigmento</span>
                            <span className="text-[11px] text-neutral-300">
                              {EYESHADOW_COLORS.find((c) => c.id === (baseAppearance.eyeshadowColor || 'bronze'))?.label || 'Bronce'}
                            </span>
                          </div>
                          <div className="flex items-center space-x-2 overflow-x-auto py-1 no-scrollbar">
                            {EYESHADOW_COLORS.map((tone) => {
                              const currentColor =
                                (baseAppearance.eyeshadow || 0) === 0 && baseAppearance.eyeshadowColor === 'none'
                                  ? 'none'
                                  : (baseAppearance.eyeshadowColor || 'bronze');
                              const isSelected = currentColor === tone.id;
                              if (tone.isNone) {
                                return (
                                  <button
                                    key={tone.id}
                                    onClick={() => updateBaseAppearance({ eyeshadowColor: 'none', eyeshadow: 0 })}
                                    className={`w-7 h-7 rounded-full border border-neutral-600 shrink-0 flex items-center justify-center transition-all ${
                                      isSelected ? 'ring-2 ring-amber-400 scale-110 bg-neutral-800' : 'opacity-70 hover:opacity-100'
                                    }`}
                                    title="Sin sombra"
                                  >
                                    <span className="text-xs text-neutral-400">⊘</span>
                                  </button>
                                );
                              }
                              return (
                                <button
                                  key={tone.id}
                                  onClick={() =>
                                    updateBaseAppearance({
                                      eyeshadowColor: tone.id,
                                      eyeshadow: (baseAppearance.eyeshadow || 0) === 0 ? 55 : baseAppearance.eyeshadow,
                                    })
                                  }
                                  style={{ backgroundColor: tone.color }}
                                  className={`w-7 h-7 rounded-full shrink-0 transition-transform ${
                                    isSelected ? 'ring-2 ring-amber-400 scale-110 shadow-lg' : 'opacity-85 hover:opacity-100'
                                  }`}
                                  title={tone.label}
                                />
                              );
                            })}
                          </div>
                        </div>

                        {/* 1.8.3 Intensidad de Sombra (Alpha) */}
                        <div className="space-y-1.5">
                          <div className="flex items-center justify-between">
                            <span className="text-[11px] text-neutral-400">Intensidad / Opacidad</span>
                            <span className="font-mono text-amber-300">{baseAppearance.eyeshadow || 0}%</span>
                          </div>
                          <input
                            type="range"
                            min="0"
                            max="100"
                            value={baseAppearance.eyeshadow || 0}
                            onChange={(e) => updateBaseAppearance({ eyeshadow: Number(e.target.value) })}
                            className="w-full h-1 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-amber-400"
                          />
                        </div>
                      </div>

                      {/* 1.9 Cejas 3D Mesh (Sprite Microblading / Ombré + Paleta + Estilos) */}
                      <div className="space-y-3 pt-2 border-t border-neutral-800/60">
                        <div className="flex items-center justify-between">
                          <span className="flex items-center space-x-2 text-neutral-200 font-medium">
                            <Sparkles className="w-3.5 h-3.5 text-orange-400" />
                            <span>Cejas (Malla 3D UV)</span>
                          </span>
                          <span className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-orange-500/10 text-orange-300 border border-orange-500/20">
                            {EYEBROW_STYLES.find((s) => s.id === (baseAppearance.eyebrowStyle || 'natural_fill'))?.label || 'Natural'}
                          </span>
                        </div>

                        {/* 1.9.1 Selector de Estilo de Cejas */}
                        <div className="grid grid-cols-2 gap-1.5">
                          {EYEBROW_STYLES.map((style) => {
                            const isSelected = (baseAppearance.eyebrowStyle || 'natural_fill') === style.id;
                            return (
                              <button
                                key={style.id}
                                onClick={() =>
                                  updateBaseAppearance({
                                    eyebrowStyle: style.id,
                                    eyebrows: (baseAppearance.eyebrows || 0) === 0 ? 55 : baseAppearance.eyebrows,
                                    eyebrowColor: (baseAppearance.eyebrowColor || 'none') === 'none' ? 'dark_brown' : baseAppearance.eyebrowColor,
                                  })
                                }
                                className={`px-2.5 py-1.5 rounded-lg border text-left transition-all ${
                                  isSelected
                                    ? 'bg-orange-500/15 border-orange-500/40 text-orange-200 font-semibold'
                                    : 'bg-[#1e1e22]/60 border-neutral-800 text-neutral-400 hover:text-neutral-200 hover:border-neutral-700'
                                }`}
                              >
                                <div className="text-[11px] font-medium leading-none mb-0.5">{style.label}</div>
                                <div className="text-[9px] text-neutral-500 truncate leading-tight">{style.desc}</div>
                              </button>
                            );
                          })}
                        </div>

                        {/* 1.9.2 Selector de Color de Cejas */}
                        <div className="space-y-1.5">
                          <div className="flex items-center justify-between">
                            <span className="text-[11px] text-neutral-400">Tono de pigmento</span>
                            <span className="text-[11px] text-neutral-300">
                              {EYEBROW_COLORS.find((c) => c.id === (baseAppearance.eyebrowColor || 'dark_brown'))?.label || 'Castaño'}
                            </span>
                          </div>
                          <div className="flex items-center space-x-2 overflow-x-auto py-1 no-scrollbar">
                            {EYEBROW_COLORS.map((tone) => {
                              const currentColor =
                                (baseAppearance.eyebrows || 0) === 0 && baseAppearance.eyebrowColor === 'none'
                                  ? 'none'
                                  : (baseAppearance.eyebrowColor || 'dark_brown');
                              const isSelected = currentColor === tone.id;
                              if (tone.isNone) {
                                return (
                                  <button
                                    key={tone.id}
                                    onClick={() => updateBaseAppearance({ eyebrowColor: 'none', eyebrows: 0 })}
                                    className={`w-7 h-7 rounded-full border border-neutral-600 shrink-0 flex items-center justify-center transition-all ${
                                      isSelected ? 'ring-2 ring-orange-400 scale-110 bg-neutral-800' : 'opacity-70 hover:opacity-100'
                                    }`}
                                    title="Sin cejas"
                                  >
                                    <span className="text-xs text-neutral-400">⊘</span>
                                  </button>
                                );
                              }
                              return (
                                <button
                                  key={tone.id}
                                  onClick={() =>
                                    updateBaseAppearance({
                                      eyebrowColor: tone.id,
                                      eyebrows: (baseAppearance.eyebrows || 0) === 0 ? 55 : baseAppearance.eyebrows,
                                    })
                                  }
                                  style={{ backgroundColor: tone.color }}
                                  className={`w-7 h-7 rounded-full shrink-0 transition-transform ${
                                    isSelected ? 'ring-2 ring-orange-400 scale-110 shadow-lg' : 'opacity-85 hover:opacity-100'
                                  }`}
                                  title={tone.label}
                                />
                              );
                            })}
                          </div>
                        </div>

                        {/* 1.9.3 Intensidad de Cejas (Alpha) */}
                        <div className="space-y-1.5">
                          <div className="flex items-center justify-between">
                            <span className="text-[11px] text-neutral-400">Intensidad / Definición</span>
                            <span className="font-mono text-orange-300">{baseAppearance.eyebrows || 0}%</span>
                          </div>
                          <input
                            type="range"
                            min="0"
                            max="100"
                            value={baseAppearance.eyebrows || 0}
                            onChange={(e) => updateBaseAppearance({ eyebrows: Number(e.target.value) })}
                            className="w-full h-1 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-orange-400"
                          />
                        </div>
                      </div>
                    </div>
                  )}


                </div>
              </div>
            )}

            {/* SUBTAB 2: MAQUILLAJE */}
            {subTab === 'maquillaje' && (
              <div className="space-y-4">
                {/* 1. Tarjeta Flotante Bigo Style: Dos Sliders Maestros + Bookmark */}
                <div className="p-3 bg-[#17171a] border border-neutral-800 rounded-2xl shadow-xl space-y-2.5">
                  <div className="flex items-center space-x-3">
                    <div className="flex-1 space-y-2">
                      {/* Slider 1: Filtro de Luz y Tono del Rostro (3 Círculos) */}
                      <div className="flex items-center space-x-2.5">
                        <div
                          className="shrink-0 text-neutral-300 hover:text-white transition-colors"
                          title="Filtro de Luz y Tono del Rostro"
                        >
                          <svg className="w-5 h-5 text-[#00e5c9]" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                            <circle cx="12" cy="7.5" r="4" />
                            <circle cx="7.5" cy="15" r="4" />
                            <circle cx="16.5" cy="15" r="4" />
                          </svg>
                        </div>
                        <div className="flex-1 relative flex items-center">
                          <input
                            type="range"
                            min="0"
                            max="100"
                            value={filterIntensity}
                            onChange={(e) => {
                              const val = Number(e.target.value);
                              setFilterIntensity(val);
                              if (activePresetConfig) {
                                applyComposite(baseAppearance, activePresetConfig, val, makeupIntensity);
                              }
                            }}
                            className="w-full h-1.5 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-[#00e5c9]"
                          />
                        </div>
                        <span className="text-[11px] font-mono text-neutral-300 w-7 text-right">
                          {filterIntensity}
                        </span>
                      </div>

                      {/* Slider 2: Maquillaje Aplicado (Carita con Destello) */}
                      <div className="flex items-center space-x-2.5">
                        <div
                          className="shrink-0 text-neutral-300 hover:text-white transition-colors"
                          title="Intensidad de Maquillaje"
                        >
                          <svg className="w-5 h-5 text-[#ff6b9d]" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                            <circle cx="11" cy="13" r="8" />
                            <path d="M8 12a1 1 0 1 0 0 .01" />
                            <path d="M14 12a1 1 0 1 0 0 .01" />
                            <path d="M8.5 15.5c.8 1 2.2 1.5 3.5 1.5s2.7-.5 3.5-1.5" />
                            <path d="M19 4v4m2-2h-4" />
                          </svg>
                        </div>
                        <div className="flex-1 relative flex items-center">
                          <input
                            type="range"
                            min="0"
                            max="100"
                            value={makeupIntensity}
                            onChange={(e) => {
                              const val = Number(e.target.value);
                              setMakeupIntensity(val);
                              if (activePresetConfig) {
                                applyComposite(baseAppearance, activePresetConfig, filterIntensity, val);
                              }
                            }}
                            className="w-full h-1.5 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-[#ff6b9d]"
                          />
                        </div>
                        <span className="text-[11px] font-mono text-neutral-300 w-7 text-right">
                          {makeupIntensity}
                        </span>
                      </div>
                    </div>

                    {/* Botón de Favorito / Bookmark del Preset Actual */}
                    <button
                      onClick={(e) => {
                        if (selectedPresetId !== 'none') {
                          toggleFavorite(selectedPresetId, e);
                        }
                      }}
                      disabled={selectedPresetId === 'none'}
                      title={
                        selectedPresetId === 'none'
                          ? 'Selecciona un preset para agregarlo a favoritos'
                          : favoriteIds.includes(selectedPresetId)
                          ? 'Quitar de favoritos'
                          : 'Guardar como favorito'
                      }
                      className={`p-2.5 rounded-xl border transition-all shrink-0 ${
                        selectedPresetId !== 'none' && favoriteIds.includes(selectedPresetId)
                          ? 'bg-amber-500/20 border-amber-400/80 text-amber-400'
                          : selectedPresetId !== 'none'
                          ? 'bg-[#222226] border-neutral-700 text-neutral-400 hover:text-white hover:border-neutral-500'
                          : 'bg-[#1e1e22] border-neutral-800 text-neutral-600 opacity-50 cursor-not-allowed'
                      }`}
                    >
                      <Bookmark
                        className={`w-5 h-5 ${
                          selectedPresetId !== 'none' && favoriteIds.includes(selectedPresetId)
                            ? 'fill-amber-400 text-amber-400'
                            : ''
                        }`}
                      />
                    </button>

                    {/* Botón Guardar nuevo archivo JSON de preset en directorio presets/ */}
                    <button
                      onClick={() => setIsSaveModalOpen(true)}
                      title="Guardar ajuste de belleza actual como archivo JSON independiente en presets/"
                      className="p-2.5 rounded-xl border border-neutral-700 bg-[#222226] text-neutral-300 hover:text-white hover:border-[#00e5c9] hover:bg-[#00e5c9]/10 transition-all shrink-0 flex items-center space-x-1 cursor-pointer"
                    >
                      <Plus className="w-4 h-4 text-[#00e5c9]" />
                      <span className="text-xs font-semibold text-[#00e5c9]">Guardar JSON</span>
                    </button>
                  </div>
                </div>

                {isSaveModalOpen && (
                  <div className="p-3.5 rounded-2xl bg-[#1e1e24] border border-[#00e5c9]/40 space-y-3 shadow-xl">
                    <div className="flex items-center justify-between">
                      <span className="text-xs font-bold text-white flex items-center space-x-1.5">
                        <Save className="w-4 h-4 text-[#00e5c9]" />
                        <span>Guardar Preset en Directorio presets/</span>
                      </span>
                      <button
                        onClick={() => setIsSaveModalOpen(false)}
                        className="text-neutral-400 hover:text-white p-1 cursor-pointer"
                      >
                        <X className="w-4 h-4" />
                      </button>
                    </div>

                    <div className="space-y-1">
                      <label className="text-[11px] text-neutral-400 font-medium">Nombre del Preset:</label>
                      <input
                        type="text"
                        value={newPresetName}
                        onChange={(e) => setNewPresetName(e.target.value)}
                        placeholder="Ej: Velvet Studio, Glam Glow..."
                        className="w-full bg-[#141416] border border-neutral-700 rounded-xl px-3 py-1.5 text-xs text-white focus:outline-none focus:border-[#00e5c9]"
                      />
                    </div>

                    <div className="space-y-1">
                      <label className="text-[11px] text-neutral-400 font-medium">Categoría:</label>
                      <select
                        value={newPresetCategory}
                        onChange={(e) => setNewPresetCategory(e.target.value)}
                        className="w-full bg-[#141416] border border-neutral-700 rounded-xl px-3 py-1.5 text-xs text-white focus:outline-none focus:border-[#00e5c9] cursor-pointer"
                      >
                        <option value="estilo_ins">Estilo Ins</option>
                        <option value="vogue">Vogue</option>
                        <option value="hombre">Hombre</option>
                      </select>
                    </div>

                    <div className="flex items-center justify-end space-x-2 pt-1">
                      <button
                        onClick={() => setIsSaveModalOpen(false)}
                        className="px-3 py-1.5 rounded-xl text-xs text-neutral-400 hover:text-white bg-neutral-800 cursor-pointer"
                      >
                        Cancelar
                      </button>
                      <button
                        onClick={handleSaveCustomPresetFile}
                        disabled={!newPresetName.trim()}
                        className="px-4 py-1.5 rounded-xl text-xs font-bold text-black bg-[#00e5c9] hover:bg-[#00c8b0] disabled:opacity-50 disabled:cursor-not-allowed shadow-md cursor-pointer"
                      >
                        Guardar Archivo .json
                      </button>
                    </div>
                  </div>
                )}

                {/* 2. Categorías (Tabs de filtro de catálogo) */}
                <div className="flex items-center space-x-1.5 overflow-x-auto pb-1 no-scrollbar text-xs">
                  {[
                    { id: 'todos', label: 'Todos' },
                    { id: 'favoritos', label: 'Favoritos' },
                    { id: 'hermoso', label: 'Hermoso' },
                    { id: 'estilo_ins', label: 'Estilo Ins' },
                    { id: 'vogue', label: 'Vogue' },
                    { id: 'hombre', label: 'Hombre' },
                  ].map((cat) => {
                    const isActive = makeupCategory === cat.id;
                    return (
                      <button
                        key={cat.id}
                        onClick={() => setMakeupCategory(cat.id)}
                        className={`px-3 py-1 rounded-full whitespace-nowrap transition-all font-medium ${
                          isActive
                            ? 'bg-[#00e5c9] text-black shadow-md'
                            : 'bg-[#1b1b1e] text-neutral-400 hover:text-white hover:bg-[#25252a]'
                        }`}
                      >
                        {cat.label}
                      </button>
                    );
                  })}
                </div>

                {/* 3. Cuadrícula de Presets (Fotos estilo Bigo) */}
                <div className="grid grid-cols-4 gap-2.5 pt-1 max-h-[380px] overflow-y-auto pr-1">
                  {/* Botón Ninguno */}
                  <button
                    onClick={() => handleSelectPreset({ id: 'none', isNone: true })}
                    className="flex flex-col items-center space-y-1.5 group"
                    title="Desactivar maquillaje"
                  >
                    <div
                      className={`w-full aspect-square rounded-2xl bg-[#1b1b1e] border flex items-center justify-center transition-all ${
                        selectedPresetId === 'none'
                          ? 'border-[#00e5c9] ring-2 ring-[#00e5c9]/40 text-[#00e5c9]'
                          : 'border-neutral-800 text-neutral-500 group-hover:text-neutral-300 group-hover:border-neutral-700'
                      }`}
                    >
                      <span className="text-xl">⊘</span>
                    </div>
                    <span
                      className={`text-[11px] truncate w-full text-center ${
                        selectedPresetId === 'none' ? 'text-[#00e5c9] font-medium' : 'text-neutral-400'
                      }`}
                    >
                      Ninguno
                    </span>
                  </button>

                  {/* Lista de Presets filtrados */}
                  {presetCatalog
                    .filter((item) => {
                      if (makeupCategory === 'favoritos') {
                        return favoriteIds.includes(item.id);
                      }
                      if (makeupCategory === 'todos') {
                        return true;
                      }
                      return item.category === makeupCategory;
                    })
                    .map((item) => {
                      const isSelected = selectedPresetId === item.id;
                      const isFav = favoriteIds.includes(item.id);

                      return (
                        <button
                          key={item.id}
                          onClick={() => handleSelectPreset(item)}
                          className="flex flex-col items-center space-y-1.5 group relative text-left"
                          title={item.name}
                        >
                          <div
                            className={`w-full aspect-square rounded-2xl overflow-hidden relative border transition-all ${
                              isSelected
                                ? 'border-[#00e5c9] ring-2 ring-[#00e5c9]/60 scale-102 shadow-lg shadow-[#00e5c9]/10'
                                : 'border-neutral-800/80 opacity-85 group-hover:opacity-100 group-hover:border-neutral-700'
                            }`}
                          >
                            <img
                              src={item.avatar}
                              alt={item.name}
                              className="w-full h-full object-cover"
                              loading="lazy"
                            />

                            {/* Badge de Favorito (cintillo dorado esquina superior izquierda) */}
                            {isFav && (
                              <div
                                className="absolute top-1 left-1 bg-amber-400 text-black p-0.5 rounded shadow z-10"
                                title="En favoritos"
                              >
                                <Bookmark className="w-2.5 h-2.5 fill-black" />
                              </div>
                            )}

                            {/* Sparkle si está seleccionado */}
                            {isSelected && (
                              <div className="absolute bottom-1 right-1 bg-[#00e5c9] text-black p-0.5 rounded-full shadow">
                                <Sparkles className="w-2.5 h-2.5 fill-black" />
                              </div>
                            )}
                          </div>

                          <span
                            className={`text-[11px] truncate w-full text-center leading-tight ${
                              isSelected ? 'text-[#00e5c9] font-medium' : 'text-neutral-300'
                            }`}
                          >
                            {item.name}
                          </span>
                        </button>
                      );
                    })}
                </div>
              </div>
            )}

            {/* SUBTAB 3: CUERPO */}
            {subTab === 'cuerpo' && (
              <div className="space-y-4">
                {/* Banner de Ayuda IA (Descartable) */}
                {showBodyBanner && (
                  <div className="p-3 bg-[#1c1c20] border border-neutral-800 rounded-xl flex items-start justify-between text-neutral-300 text-xs">
                    <p className="leading-relaxed pr-2">
                      Pon todo el cuerpo dentro del encuadre y mantén el fondo despejado
                    </p>
                    <button
                      onClick={() => setShowBodyBanner(false)}
                      className="text-neutral-400 hover:text-white p-0.5 rounded transition-colors shrink-0"
                    >
                      <X className="w-4 h-4" />
                    </button>
                  </div>
                )}

                {/* 1. Contorneado corporal */}
                <div className="space-y-1.5 pt-1">
                  <div className="flex items-center justify-between">
                    <span className="flex items-center space-x-2 text-neutral-200 font-medium">
                      {/* Icono Silueta Corporal */}
                      <svg className="w-4 h-4 text-neutral-300" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                        <path d="M8 4a4 4 0 0 1 8 0v3c2 2 2 4 1 7l-2 6H9l-2-6c-1-3-1-5 1-7V4Z" />
                      </svg>
                      <span>Contorneado corporal</span>
                    </span>
                    <span className="font-mono text-neutral-300">{contornoCorporal}</span>
                  </div>
                  <input
                    type="range"
                    min="0"
                    max="100"
                    value={contornoCorporal}
                    onChange={(e) => setContornoCorporal(Number(e.target.value))}
                    className="w-full h-1 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-white"
                  />
                </div>

                {/* 2. Contorneado de brazos */}
                <div className="space-y-1.5">
                  <div className="flex items-center justify-between">
                    <span className="flex items-center space-x-2 text-neutral-200 font-medium">
                      {/* Icono Brazos */}
                      <svg className="w-4 h-4 text-neutral-300" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                        <path d="M4 14c2-4 6-6 10-4l4 2c1 1 2 3 1 5l-1 2-5-1-4 2-5-6Z" />
                      </svg>
                      <span>Contorneado de brazos</span>
                    </span>
                    <span className="font-mono text-neutral-300">{contornoBrazos}</span>
                  </div>
                  <input
                    type="range"
                    min="0"
                    max="100"
                    value={contornoBrazos}
                    onChange={(e) => setContornoBrazos(Number(e.target.value))}
                    className="w-full h-1 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-white"
                  />
                </div>

                {/* 3. Cuello y hombros (Bipolar con centro en 0) */}
                <div className="space-y-1.5">
                  <div className="flex items-center justify-between">
                    <span className="flex items-center space-x-2 text-neutral-200 font-medium">
                      {/* Icono Cuello y hombros */}
                      <svg className="w-4 h-4 text-neutral-300" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                        <path d="M4 18c2-4 5-6 8-6s6 2 8 6" />
                        <path d="M10 8v4M14 8v4" />
                      </svg>
                      <span>Cuello y hombros</span>
                    </span>
                    <span className="font-mono text-neutral-300">{cuelloHombros}</span>
                  </div>
                  <input
                    type="range"
                    min="-50"
                    max="50"
                    value={cuelloHombros}
                    onChange={(e) => setCuelloHombros(Number(e.target.value))}
                    className="w-full h-1 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-white"
                  />
                </div>

                {/* 4. Longitud de las piernas */}
                <div className="space-y-3 pt-1">
                  <div className="flex items-center space-x-2 text-neutral-200 font-medium">
                    {/* Icono Piernas */}
                    <svg className="w-4 h-4 text-neutral-300" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                      <path d="M8 4v16M16 4v16" />
                    </svg>
                    <span>Longitud de las piernas</span>
                  </div>

                  {/* Sub-slider Rango */}
                  <div className="pl-6 space-y-1">
                    <div className="flex items-center justify-between">
                      <span className="text-neutral-400">Rango</span>
                      <span className="font-mono text-neutral-300">{piernasRango}</span>
                    </div>
                    <input
                      type="range"
                      min="0"
                      max="100"
                      value={piernasRango}
                      onChange={(e) => setPiernasRango(Number(e.target.value))}
                      className="w-full h-1 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-white"
                    />
                  </div>

                  {/* Sub-slider Solidez */}
                  <div className="pl-6 space-y-1">
                    <div className="flex items-center justify-between">
                      <span className="text-neutral-400">Solidez</span>
                      <span className="font-mono text-neutral-300">{piernasSolidez}</span>
                    </div>
                    <input
                      type="range"
                      min="0"
                      max="100"
                      value={piernasSolidez}
                      onChange={(e) => setPiernasSolidez(Number(e.target.value))}
                      className="w-full h-1 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-white"
                    />
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* TAB: LUZ */}
      {mainTab === 'luz' && (
        <div className="flex-1 flex flex-col overflow-y-auto p-5 space-y-5">
          <div className="flex items-center justify-between pb-2 border-b border-neutral-800">
            <span className="font-semibold text-neutral-200">Ajustes de Iluminación</span>
            <button
              onClick={() => onChange({
                ...effects,
                exposure: 0.0,
                whiteBalance: 0.0,
                brightness: 0.0,
                aiAutoTuneEnabled: true,
                aiLightingEnabled: true,
                aiDenoiseEnabled: true,
              })}
              className="text-neutral-400 hover:text-white flex items-center space-x-1 text-[11px]"
            >
              <RotateCcw className="w-3 h-3" />
              <span>Restablecer</span>
            </button>
          </div>

          {/* AI Switches */}
          <div className="space-y-3 bg-[#18181b] p-3.5 rounded-xl border border-neutral-800">
            <div className="flex items-center justify-between">
              <div>
                <div className="text-xs font-medium text-white">Auto-Tune Fotométrico</div>
                <div className="text-[10px] text-neutral-400">Calibración continua de exposición y color</div>
              </div>
              <input
                type="checkbox"
                checked={effects.aiAutoTuneEnabled}
                onChange={(e) => onChange({ ...effects, aiAutoTuneEnabled: e.target.checked })}
                className="w-4 h-4 accent-pink-500 rounded cursor-pointer"
              />
            </div>

            <div className="flex items-center justify-between">
              <div>
                <div className="text-xs font-medium text-white">Iluminación de Estudio IA</div>
                <div className="text-[10px] text-neutral-400">Relleno suave de sombras faciales</div>
              </div>
              <input
                type="checkbox"
                checked={effects.aiLightingEnabled}
                onChange={(e) => onChange({ ...effects, aiLightingEnabled: e.target.checked })}
                className="w-4 h-4 accent-pink-500 rounded cursor-pointer"
              />
            </div>

            <div className="flex items-center justify-between">
              <div>
                <div className="text-xs font-medium text-white">Reducción de Ruido IA</div>
                <div className="text-[10px] text-neutral-400">Filtro bilateral 1440p de grano fino</div>
              </div>
              <input
                type="checkbox"
                checked={effects.aiDenoiseEnabled}
                onChange={(e) => onChange({ ...effects, aiDenoiseEnabled: e.target.checked })}
                className="w-4 h-4 accent-pink-500 rounded cursor-pointer"
              />
            </div>
          </div>

          {/* Sliders Manuales de Luz */}
          <div className="space-y-4">
            {/* Exposición */}
            <div className="space-y-1.5">
              <div className="flex items-center justify-between">
                <span className="text-neutral-200 font-medium">Exposición</span>
                <span className="font-mono text-neutral-300">{(effects.exposure || 0).toFixed(2)}</span>
              </div>
              <input
                type="range"
                min="-1.0"
                max="1.0"
                step="0.05"
                value={effects.exposure ?? 0.0}
                onChange={(e) => onChange({ ...effects, exposure: parseFloat(e.target.value) })}
                className="w-full h-1 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-white"
              />
            </div>

            {/* Balance de Blancos */}
            <div className="space-y-1.5">
              <div className="flex items-center justify-between">
                <span className="text-neutral-200 font-medium">Balance de Blancos (Kelvin)</span>
                <span className="font-mono text-neutral-300">{(effects.whiteBalance || 0).toFixed(2)}</span>
              </div>
              <input
                type="range"
                min="-1.0"
                max="1.0"
                step="0.05"
                value={effects.whiteBalance ?? 0.0}
                onChange={(e) => onChange({ ...effects, whiteBalance: parseFloat(e.target.value) })}
                className="w-full h-1 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-white"
              />
              <div className="flex justify-between text-[10px] text-neutral-500">
                <span>Frío (Azul)</span>
                <span>Neutro</span>
                <span>Cálido (Ámbar)</span>
              </div>
            </div>

            {/* Brillo */}
            <div className="space-y-1.5">
              <div className="flex items-center justify-between">
                <span className="text-neutral-200 font-medium">Brillo de Piso</span>
                <span className="font-mono text-neutral-300">{(effects.brightness || 0).toFixed(2)}</span>
              </div>
              <input
                type="range"
                min="-0.30"
                max="0.30"
                step="0.02"
                value={effects.brightness ?? 0.0}
                onChange={(e) => onChange({ ...effects, brightness: parseFloat(e.target.value) })}
                className="w-full h-1 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-white"
              />
            </div>
          </div>
        </div>
      )}

      {/* TAB: FILTROS */}
      {mainTab === 'filtros' && (
        <div className="flex-1 flex flex-col overflow-y-auto p-5 space-y-4">
          {/* Cabecera */}
          <div className="flex items-center justify-between pb-2 border-b border-neutral-800">
            <div className="flex items-center space-x-2">
              <Film className="w-4 h-4 text-[#00e5c9]" />
              <span className="font-semibold text-neutral-200">Filtros Cinematográficos</span>
            </div>
            <button
              onClick={() =>
                onChange({
                  ...effects,
                  filterLut: 'none',
                  filterIntensity: 0.80,
                })
              }
              className="text-neutral-400 hover:text-white flex items-center space-x-1 text-[11px] transition-colors"
            >
              <RotateCcw className="w-3 h-3" />
              <span>Restablecer</span>
            </button>
          </div>

          {/* Slider de Intensidad del Filtro (visible siempre que haya un filtro activo) */}
          {effects.filterLut && effects.filterLut !== 'none' && (
            <div className="bg-[#18181b]/90 border border-neutral-800 rounded-xl p-3 space-y-2 shadow-sm">
              <div className="flex items-center justify-between">
                <span className="flex items-center space-x-1.5 text-xs text-neutral-300 font-medium">
                  <Sliders className="w-3.5 h-3.5 text-[#00e5c9]" />
                  <span>
                    Intensidad:{' '}
                    <span className="text-[#00e5c9]">
                      {FILTER_LUTS.find((f) => f.id === effects.filterLut)?.name || effects.filterLut}
                    </span>
                  </span>
                </span>
                <span className="font-mono text-xs text-neutral-200">
                  {Math.round((effects.filterIntensity ?? 0.80) * 100)}%
                </span>
              </div>
              <div className="flex items-center space-x-3">
                <input
                  type="range"
                  min="0"
                  max="100"
                  step="1"
                  value={Math.round((effects.filterIntensity ?? 0.80) * 100)}
                  onChange={(e) =>
                    onChange({
                      ...effects,
                      filterIntensity: parseFloat(e.target.value) / 100.0,
                    })
                  }
                  className="w-full h-1.5 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-[#00e5c9]"
                />
              </div>
            </div>
          )}

          {/* Barra de Categorías Estilo Bigo / TikTok */}
          <div className="flex items-center space-x-1.5 overflow-x-auto pb-1 no-scrollbar text-xs">
            {[
              { id: 'todos', label: 'Todos' },
              { id: 'natural', label: 'Natural' },
              { id: 'emocional', label: 'Emocional' },
              { id: 'vivido', label: 'Vívido' },
              { id: 'onirico', label: 'Onírico' },
              { id: 'vintage', label: 'Vintage' },
            ].map((cat) => {
              const isActive = filterCategory === cat.id;
              return (
                <button
                  key={cat.id}
                  onClick={() => setFilterCategory(cat.id)}
                  className={`px-3 py-1 rounded-full whitespace-nowrap transition-all font-medium ${
                    isActive
                      ? 'bg-[#00e5c9] text-black shadow-md'
                      : 'bg-[#1b1b1e] text-neutral-400 hover:text-white hover:bg-[#25252a]'
                  }`}
                >
                  {cat.label}
                </button>
              );
            })}
          </div>

          {/* Cuadrícula de Filtros / LUTs */}
          <div className="grid grid-cols-4 gap-2.5 pt-1 max-h-[420px] overflow-y-auto pr-1">
            {/* Opción: Ninguno */}
            <button
              onClick={() =>
                onChange({
                  ...effects,
                  filterLut: 'none',
                })
              }
              className="flex flex-col items-center space-y-1.5 group"
              title="Sin filtro de color"
            >
              <div
                className={`w-full aspect-square rounded-2xl bg-[#1b1b1e] border flex items-center justify-center transition-all ${
                  !effects.filterLut || effects.filterLut === 'none'
                    ? 'border-[#00e5c9] ring-2 ring-[#00e5c9]/40 text-[#00e5c9] shadow-md'
                    : 'border-neutral-800 text-neutral-500 group-hover:text-neutral-300 group-hover:border-neutral-700'
                }`}
              >
                <span className="text-xl">⊘</span>
              </div>
              <span
                className={`text-[11px] truncate w-full text-center ${
                  !effects.filterLut || effects.filterLut === 'none'
                    ? 'text-[#00e5c9] font-medium'
                    : 'text-neutral-400'
                }`}
              >
                Sin Filtro
              </span>
            </button>

            {/* Lista de Filtros de la Categoría Seleccionada */}
            {FILTER_LUTS.filter((lut) => filterCategory === 'todos' || lut.category === filterCategory).map(
              (lut) => {
                const isSelected = effects.filterLut === lut.id;
                return (
                  <button
                    key={lut.id}
                    onClick={() =>
                      onChange({
                        ...effects,
                        filterLut: isSelected ? 'none' : lut.id,
                        filterIntensity: effects.filterIntensity ?? 0.80,
                      })
                    }
                    className="flex flex-col items-center space-y-1.5 group relative"
                    title={`${lut.name} - ${lut.desc}`}
                  >
                    <div
                      style={{ background: lut.gradient }}
                      className={`w-full aspect-square rounded-2xl border relative overflow-hidden transition-all duration-150 ${
                        isSelected
                          ? 'border-white ring-2 ring-[#00e5c9] ring-offset-2 ring-offset-[#121214] scale-105 shadow-[0_0_14px_rgba(0,229,201,0.4)]'
                          : 'border-neutral-800 opacity-90 group-hover:opacity-100 group-hover:scale-102 group-hover:border-neutral-600'
                      }`}
                    >
                      {/* Brillo sutil y viñeta interna */}
                      <div className="absolute inset-0 bg-gradient-to-t from-black/40 via-transparent to-white/20 pointer-events-none" />

                      {/* Badge con Check si está seleccionado */}
                      {isSelected && (
                        <div className="absolute top-1.5 right-1.5 w-4 h-4 rounded-full bg-[#00e5c9] text-black flex items-center justify-center shadow-md">
                          <Check className="w-2.5 h-2.5 stroke-[3]" />
                        </div>
                      )}
                    </div>

                    <span
                      className={`text-[11px] truncate w-full text-center transition-colors ${
                        isSelected
                          ? 'text-[#00e5c9] font-medium'
                          : 'text-neutral-300 group-hover:text-white'
                      }`}
                    >
                      {lut.name}
                    </span>
                  </button>
                );
              }
            )}
          </div>

          {/* Información técnica y separación de capas */}
          <div className="pt-2 text-[11px] text-neutral-500 border-t border-neutral-800/60 leading-relaxed">
            <p>
              ✨ Gradación de color master post-procesado. Aplica como capa superior final sin afectar el contraste de la cámara ni los filtros de belleza.
            </p>
          </div>
        </div>
      )}

      {/* TAB: FONDO */}
      {mainTab === 'fondo' && (
        <div className="flex-1 flex flex-col overflow-y-auto p-5 space-y-5">
          <div className="flex items-center justify-between pb-2 border-b border-neutral-800">
            <span className="font-semibold text-neutral-200">Segmentación de Fondo</span>
            <button
              onClick={() => onChange({
                ...effects,
                bgMode: 0,
                blurRadius: 16.0,
                edgeFeather: 1.5,
              })}
              className="text-neutral-400 hover:text-white flex items-center space-x-1 text-[11px]"
            >
              <RotateCcw className="w-3 h-3" />
              <span>Restablecer</span>
            </button>
          </div>

          {/* Selector de Modo */}
          <div className="grid grid-cols-3 gap-2">
            {[
              { id: 0, label: 'Ninguno' },
              { id: 1, label: 'Desenfoque' },
              { id: 3, label: 'Verde' },
            ].map((m) => (
              <button
                key={m.id}
                onClick={() => onChange({ ...effects, bgMode: m.id as any })}
                className={`py-2 px-3 rounded-lg border text-xs font-medium transition-all ${
                  effects.bgMode === m.id
                    ? 'border-white bg-white/10 text-white shadow'
                    : 'border-neutral-800 bg-[#161618] text-neutral-400 hover:text-white'
                }`}
              >
                {m.label}
              </button>
            ))}
          </div>

          {effects.bgMode !== 0 && (
            <div className="space-y-4 pt-2">
              <div className="space-y-1.5">
                <div className="flex items-center justify-between">
                  <span className="text-neutral-200 font-medium">Radio de Desenfoque</span>
                  <span className="font-mono text-neutral-300">{Math.round(effects.blurRadius || 16)} px</span>
                </div>
                <input
                  type="range"
                  min="2"
                  max="40"
                  value={effects.blurRadius ?? 16}
                  onChange={(e) => onChange({ ...effects, blurRadius: parseFloat(e.target.value) })}
                  className="w-full h-1 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-white"
                />
              </div>

              <div className="space-y-1.5">
                <div className="flex items-center justify-between">
                  <span className="text-neutral-200 font-medium">Suavizado de Bordes</span>
                  <span className="font-mono text-neutral-300">{(effects.edgeFeather || 1.5).toFixed(1)}</span>
                </div>
                <input
                  type="range"
                  min="0.5"
                  max="8.0"
                  step="0.5"
                  value={effects.edgeFeather ?? 1.5}
                  onChange={(e) => onChange({ ...effects, edgeFeather: parseFloat(e.target.value) })}
                  className="w-full h-1 bg-[#28282c] rounded-lg appearance-none cursor-pointer accent-white"
                />
              </div>
            </div>
          )}
        </div>
      )}

      {/* TAB: EFECTOS (Halloween, Props 3D, Máscaras) */}
      {mainTab === 'efectos' && (
        <div className="flex-1 flex flex-col min-h-0 overflow-y-auto px-4 py-4 space-y-5 custom-scrollbar">
          {/* Cabecera con botón para Cancelar Todos los Efectos */}
          <div className="flex items-center justify-between">
            <div>
              <h3 className="text-xs font-semibold text-neutral-200 uppercase tracking-wider flex items-center gap-1.5">
                <Sparkles className="w-3.5 h-3.5 text-red-500" />
                Accesorios & Halloween
              </h3>
              <p className="text-[11px] text-neutral-500 mt-0.5">
                Efectos 3D y props anclados a la cabeza en tiempo real
              </p>
            </div>
            {effects.activeEffect && effects.activeEffect !== 'none' && (
              <button
                onClick={() => onChange({ ...effects, activeEffect: 'none' })}
                className="flex items-center gap-1 px-2.5 py-1 text-[11px] font-medium text-red-400 bg-red-950/40 hover:bg-red-900/50 border border-red-800/60 rounded-lg transition-all"
                title="Cancelar todos los efectos activos"
              >
                <X className="w-3 h-3" />
                <span>Cancelar</span>
              </button>
            )}
          </div>

          {/* Grid de Efectos y Accesorios */}
          <div className="grid grid-cols-2 gap-2.5">
            {/* Botón 1: Ninguno (Sin Efecto) */}
            <button
              onClick={() => onChange({ ...effects, activeEffect: 'none' })}
              className={`flex flex-col items-center justify-center p-3 rounded-xl border text-center transition-all ${
                !effects.activeEffect || effects.activeEffect === 'none'
                  ? 'bg-neutral-800/80 border-neutral-600 text-white shadow-md'
                  : 'bg-neutral-900/40 border-neutral-800/80 text-neutral-400 hover:bg-neutral-800/50 hover:border-neutral-700'
              }`}
            >
              <div className="w-16 h-16 rounded-lg bg-neutral-950/60 border border-neutral-800 flex items-center justify-center mb-2">
                <X className="w-6 h-6 text-neutral-500" />
              </div>
              <span className="text-xs font-medium">Ninguno</span>
              <span className="text-[10px] text-neutral-500 mt-0.5">Sin accesorios</span>
            </button>

            {/* Listado dinámico de Efectos Instalados */}
            {installedEffects.map((item) => {
              const isSelected = effects.activeEffect === item.id;
              return (
                <button
                  key={item.id}
                  onClick={() => {
                    if (isSelected) {
                      onChange({ ...effects, activeEffect: 'none' });
                    } else {
                      const next = { ...effects, activeEffect: item.id };
                      if (item.variables) {
                        item.variables.forEach((v) => {
                          if ((next as any)[v.key] === undefined) {
                            (next as any)[v.key] = v.default;
                          }
                        });
                      }
                      onChange(next);
                    }
                  }}
                  className={`relative flex flex-col items-center justify-center p-3 rounded-xl border text-center transition-all ${
                    isSelected
                      ? 'bg-neutral-800/80 border-amber-500/80 text-white shadow-lg ring-1 ring-amber-500/50'
                      : 'bg-neutral-900/40 border-neutral-800/80 text-neutral-400 hover:bg-neutral-800/50 hover:border-neutral-700'
                  }`}
                >
                  <div className="relative w-16 h-16 rounded-lg overflow-hidden bg-neutral-950 border border-neutral-800 flex items-center justify-center mb-2">
                    <img
                      src={item.thumb}
                      alt={item.name}
                      className="w-full h-full object-cover"
                    />
                  </div>
                  <span className="text-xs font-medium text-neutral-200">{item.name}</span>
                  {item.badge && (
                    <span className="text-[10px] text-neutral-400 mt-0.5 font-medium">{item.badge}</span>
                  )}
                </button>
              );
            })}
          </div>

          {/* Generación Dinámica de Sliders para el Efecto Activo */}
          {(() => {
            const activeItem = installedEffects.find((e) => e.id === effects.activeEffect);
            if (!activeItem || !activeItem.variables || activeItem.variables.length === 0) {
              return null;
            }

            return (
              <div className="p-3.5 bg-neutral-900/60 border border-neutral-800/80 rounded-xl space-y-4">
                <div className="flex items-center justify-between text-xs border-b border-neutral-800/80 pb-2">
                  <span className="text-amber-400 font-medium">Ajustes: {activeItem.name}</span>
                  <span className="text-neutral-500 font-mono text-[10px]">CONFIGURABLE</span>
                </div>

                {activeItem.variables.map((v) => {
                  const rawVal = (effects as any)[v.key] ?? v.default;
                  const isPercent = v.unit === '%';
                  const displayVal = isPercent
                    ? `${Math.round(rawVal * 100)}%`
                    : v.unit === 'x'
                    ? `${Number(rawVal).toFixed(v.step < 0.1 ? 2 : 1)}x`
                    : `${rawVal}${v.unit || ''}`;

                  const minVal = isPercent ? v.min * 100 : v.min;
                  const maxVal = isPercent ? v.max * 100 : v.max;
                  const stepVal = isPercent ? v.step * 100 : v.step;
                  const currentSliderVal = isPercent ? Math.round(rawVal * 100) : rawVal;

                  return (
                    <div key={v.key} className="space-y-1.5">
                      <div className="flex items-center justify-between text-xs">
                        <span className="text-neutral-300">{v.label}</span>
                        <span className="text-neutral-400 font-mono text-[11px]">{displayVal}</span>
                      </div>
                      <input
                        type="range"
                        min={minVal}
                        max={maxVal}
                        step={stepVal}
                        value={currentSliderVal}
                        onChange={(e) => {
                          const num = Number(e.target.value);
                          const finalVal = isPercent ? num / 100.0 : num;
                          onChange({ ...effects, [v.key]: finalVal });
                        }}
                        className="w-full accent-amber-500 bg-neutral-800 h-1.5 rounded-lg appearance-none cursor-pointer"
                      />
                    </div>
                  );
                })}
              </div>
            );
          })()}
        </div>
      )}

      {/* Placeholder para tab: Ambiente */}
      {mainTab === 'ambiente' && (
        <div className="flex-1 flex flex-col items-center justify-center p-6 text-center text-neutral-500 space-y-2">
          <div className="w-12 h-12 rounded-2xl bg-neutral-900 border border-neutral-800 flex items-center justify-center text-neutral-400 mb-1">
            <Clapperboard className="w-6 h-6" />
          </div>
          <h3 className="font-semibold text-neutral-300 text-sm capitalize">{mainTab}</h3>
          <p className="text-[11px] text-neutral-500 max-w-[200px]">
            Sección de presets artísticos en preparación.
          </p>
        </div>
      )}
    </aside>
  );
};
