struct VSInput
{
    float3 position: POSITION;
    float3 normal: NORMAL;
    float3 color: COLOR0;
    column_major matrix Model: MODEL;
    float3 colour: COLOUR;
     uint        instId  : SV_InstanceID;
};

cbuffer ConstantBuffer : register(b0)
{
  column_major matrix View;
  column_major matrix Projection;
}

struct VSOutput
{
    float4 position: SV_Position;
    float3 normal: NORMAL;
    float3 color: COLOR0;
};

VSOutput VSMain(VSInput input)
{
    VSOutput output = (VSOutput)0;
    float4 VertPos = float4(input.position, 1.0);

    float4 Transform = mul(Projection, mul(View, mul(input.Model, VertPos)));
    output.position = Transform;
    output.color = input.color * input.colour;
    output.normal = input.normal;
    return output;
}


struct PSInput
{
    float4 position: SV_Position;
    float3 normal: NORMAL;
    float3 color: COLOR0;
};

struct PSOutput
{
    float4 color: SV_Target0;
};

PSOutput PSMain(PSInput input)
{
    PSOutput output = (PSOutput)0;
    output.color = float4(input.color, 1.0);
    return output;
}
