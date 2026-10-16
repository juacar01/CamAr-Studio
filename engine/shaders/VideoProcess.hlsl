// Pixel Shader completo para CamAr Studio
Texture2D g_MainTexture : register(t0);          // Video crudo de cámara
Texture2D g_SegMaskTexture : register(t1);       // Máscara de segmentación AI (Alpha/Body)
Texture2D g_VirtualBgTexture : register(t2);     // Fondo virtual personalizado
SamplerState g_LinearSampler : register(s0);

cbuffer EffectBuffer : register(b0) {
    float g_Brightness;         // -1.0 a 1.0
    float g_Contrast;           // 0.0 a 2.0
    float g_Saturation;         // 0.0 a 2.0
    float g_Exposure;           // -2.0 a 2.0
    float g_Temperature;        // -1.0 a 1.0 (Color balance)
    float g_SkinSmooth;         // 0.0 a 1.0
    int   g_BackgroundMode;     // 0: None, 1: Blur, 2: Image, 3: GreenScreen
    float g_BlurRadius;         // 1.0 a 40.0
    float2 g_Resolution;        // Ancho y alto en píxeles (ej. 1920, 1080)
    float g_SegThreshold;       // 0.0 a 1.0
    float g_EdgeFeather;        // 1.0 a 5.0
};

struct PS_INPUT {
    float4 pos : SV_POSITION;
    float2 tex : TEXCOORD0;
};

// Conversión RGB a HSV y viceversa para saturación precisa
float3 RgbToHsv(float3 c) {
    float4 K = float4(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
    float4 p = lerp(float4(c.bg, K.wz), float4(c.gb, K.xy), step(c.b, c.g));
    float4 q = lerp(float4(p.xyw, c.r), float4(c.r, p.yzx), step(p.x, c.r));
    float d = q.x - min(q.w, q.y);
    float e = 1.0e-10;
    return float3(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}

float3 HsvToRgb(float3 c) {
    float4 K = float4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    float3 p = abs(frac(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * lerp(K.xxx, clamp(p - K.xxx, 0.0, 1.0), c.y);
}

// Filtro bilateral rápido para suavizado de piel preservando bordes
float4 ApplySmartBeautify(float2 uv, float smoothStrength) {
    float4 center = g_MainTexture.Sample(g_LinearSampler, uv);
    if (smoothStrength <= 0.01f) return center;

    float2 texelSize = 1.0f / g_Resolution;
    float4 sum = center;
    float totalWeight = 1.0f;

    // Kernel 5x5 selectivo
    [unroll]
    for (int x = -2; x <= 2; x += 1) {
        [unroll]
        for (int y = -2; y <= 2; y += 1) {
            if (x == 0 && y == 0) continue;
            float2 offset = float2(x, y) * texelSize * 1.5f;
            float4 sampleCol = g_MainTexture.Sample(g_LinearSampler, uv + offset);

            // Peso espacial
            float spatialDist = dot(float2(x, y), float2(x, y));
            float spatialWeight = exp(-spatialDist / 4.0f);

            // Peso de rango (diferencia de color para no borrar bordes de ojos/labios)
            float colorDiff = distance(sampleCol.rgb, center.rgb);
            float rangeWeight = exp(-colorDiff * colorDiff * 25.0f);

            float w = spatialWeight * rangeWeight;
            sum += sampleCol * w;
            totalWeight += w;
        }
    }

    float4 smoothed = sum / totalWeight;
    return lerp(center, smoothed, smoothStrength);
}

// Bokeh Gaussiano de 13 taps para desenfoque de fondo en tiempo real
float4 GetBlurredBackground(float2 uv, float radius) {
    float2 texel = (1.0f / g_Resolution) * (radius * 0.25f);
    float4 col = float4(0, 0, 0, 0);

    col += g_MainTexture.Sample(g_LinearSampler, uv + float2(-2, -2) * texel) * 0.05f;
    col += g_MainTexture.Sample(g_LinearSampler, uv + float2( 0, -2) * texel) * 0.09f;
    col += g_MainTexture.Sample(g_LinearSampler, uv + float2( 2, -2) * texel) * 0.05f;

    col += g_MainTexture.Sample(g_LinearSampler, uv + float2(-2,  0) * texel) * 0.09f;
    col += g_MainTexture.Sample(g_LinearSampler, uv + float2( 0,  0) * texel) * 0.44f;
    col += g_MainTexture.Sample(g_LinearSampler, uv + float2( 2,  0) * texel) * 0.09f;

    col += g_MainTexture.Sample(g_LinearSampler, uv + float2(-2,  2) * texel) * 0.05f;
    col += g_MainTexture.Sample(g_LinearSampler, uv + float2( 0,  2) * texel) * 0.09f;
    col += g_MainTexture.Sample(g_LinearSampler, uv + float2( 2,  2) * texel) * 0.05f;

    return col;
}

float4 PSMain(PS_INPUT input) : SV_TARGET {
    float2 uv = input.tex;

    // 1. Obtener color con suavizado de piel inteligente
    float4 personColor = ApplySmartBeautify(uv, g_SkinSmooth);

    // 2. Corrección de Color & Iluminación
    // Exposición y Brillo
    personColor.rgb *= pow(2.0f, g_Exposure);
    personColor.rgb += g_Brightness;

    // Contraste centrado en 0.5
    personColor.rgb = ((personColor.rgb - 0.5f) * max(g_Contrast, 0.0f)) + 0.5f;

    // Temperatura de color (balance cálido/frío)
    personColor.r += g_Temperature * 0.15f;
    personColor.b -= g_Temperature * 0.15f;

    // Saturación en espacio HSV
    float3 hsv = RgbToHsv(clamp(personColor.rgb, 0.0f, 1.0f));
    hsv.y = clamp(hsv.y * g_Saturation, 0.0f, 1.0f);
    personColor.rgb = HsvToRgb(hsv);

    // 3. Segmentación de Personas y Mezcla de Fondo
    if (g_BackgroundMode != 0) {
        // Leer máscara de IA (Canal Rojo representa la probabilidad de ser persona)
        float mask = g_SegMaskTexture.Sample(g_LinearSampler, uv).r;

        // Suavizado de bordes (Feathering) para evitar cortes bruscos
        float lowEdge = max(0.0f, g_SegThreshold - (g_EdgeFeather * 0.05f));
        float highEdge = min(1.0f, g_SegThreshold + (g_EdgeFeather * 0.05f));
        float alpha = smoothstep(lowEdge, highEdge, mask);

        float4 bgColor = float4(0, 0, 0, 1);

        if (g_BackgroundMode == 1) {
            // MODO 1: Bokeh / Desenfoque de fondo
            bgColor = GetBlurredBackground(uv, g_BlurRadius);
        } else if (g_BackgroundMode == 2) {
            // MODO 2: Imagen de fondo virtual
            bgColor = g_VirtualBgTexture.Sample(g_LinearSampler, uv);
        } else if (g_BackgroundMode == 3) {
            // MODO 3: Pantalla verde pura (Chroma Key para OBS/producción)
            bgColor = float4(0.0f, 1.0f, 0.0f, 1.0f);
        }

        // Mezcla alfa final entre persona y fondo
        float4 finalColor = lerp(bgColor, personColor, alpha);
        return finalColor;
    }

    return float4(clamp(personColor.rgb, 0.0f, 1.0f), 1.0f);
}
