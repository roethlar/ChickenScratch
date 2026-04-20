using YamlDotNet.Serialization;
using YamlDotNet.Serialization.NamingConventions;

namespace ChickenScratch.Core.IO;

internal static class YamlHelper
{
    private static readonly ISerializer Serializer = new SerializerBuilder()
        .WithNamingConvention(UnderscoredNamingConvention.Instance)
        .Build();

    private static readonly IDeserializer Deserializer = new DeserializerBuilder()
        .WithNamingConvention(UnderscoredNamingConvention.Instance)
        .IgnoreUnmatchedProperties()
        .Build();

    public static string Serialize<T>(T obj) => Serializer.Serialize(obj);

    public static T Deserialize<T>(string yaml) => Deserializer.Deserialize<T>(yaml);
}
