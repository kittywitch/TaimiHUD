struct VSInput
{
    float3 position: POSITION;
    float3 normal: NORMAL;
    float3 color: COLOR0;
    float2 texture: TEXCOORD0,
};

Texture2D meshTexture: register(t0);
SamplerState samLinear : register(s0);

cbuffer ConstantBuffer : register(b0)
{
  column_major matrix Model;
  column_major matrix View;
  column_major matrix Projection;
}

struct VSOutput
{
    float4 position: SV_Position;
    float3 normal: NORMAL;
    float3 color: COLOR0;
    float2 texture: TEXCOORD0,
};

VSOutput VSMain(VSInput input)
{
    VSOutput output = (VSOutput)0;
    float4 VertPos = float4(input.position, 1.0);

    float4 Transform = mul(Projection, mul(View, mul(Model, VertPos)));
    output.position = Transform;
    output.color = input.color;
    output.normal = input.normal;
    output.texture = input.texture;
    return output;
}


struct PSInput
{
    float4 position: SV_Position;
    float3 normal: NORMAL;
    float3 color: COLOR0;
    float2 texture: TEXCOORD0,
};

struct PSOutput
{
    float4 color: SV_Target0;
};

PSOutput PSMain(PSInput input)
{
    PSOutput output = (PSOutput)0;
    float4 Multiplier = float4(input.color, 1.0);
    float4 vDiffuse = meshTexture.Sample(samLinear, input.texture);
    output.color = vDiffuse * Multiplier;
    return output;
}
