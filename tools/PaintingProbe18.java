import net.minecraft.entity.decoration.painting.PaintingEntity;

/**
 * Dump 1.8.9's 26 painting motives.
 *
 * <p>1.8 has no {@code painting_variant} registry: {@code PaintingEntity.PaintingMotive}
 * is an enum of name, pixel size and texture offset, and the enum constant is
 * the motive a saved painting carries. Each line is:
 *
 * <pre>
 * id &lt;tab&gt; name &lt;tab&gt; width &lt;tab&gt; height &lt;tab&gt; textureX &lt;tab&gt; textureY
 * </pre>
 *
 * <p>The Rust generator maps the pixel size to block units, writes the
 * current {@code data/minecraft/painting_variant/*.json} files and lists every
 * id in the {@code #placeable} tag. The 1.8 client jar's own
 * {@code textures/painting/*.png} supply the art.
 *
 * <pre>
 * javac -cp "$JAR" tools/PaintingProbe18.java
 * java -cp "$JAR:." PaintingProbe18 &gt; paintings-1.8.9.txt
 * </pre>
 */
public class PaintingProbe18 {
    public static void main(String[] args) throws Exception {
        Class<?> motives = Class.forName(
            "net.minecraft.entity.decoration.painting.PaintingEntity$PaintingMotive");
        Object[] values = motives.getEnumConstants();
        // `Enum.name()` is the constant name; the public `name` field is the
        // motive string 1.8 saves (already lowercase with underscores).
        for (Object motive : values) {
            String id = ((Enum<?>) motive).name().toLowerCase();
            java.lang.reflect.Field name = motives.getField("name");
            java.lang.reflect.Field width = motives.getField("width");
            java.lang.reflect.Field height = motives.getField("height");
            java.lang.reflect.Field textureX = motives.getField("textureX");
            java.lang.reflect.Field textureY = motives.getField("textureY");
            System.out.println(id + "\t" + name.get(motive) + "\t" + width.getInt(motive)
                + "\t" + height.getInt(motive) + "\t" + textureX.getInt(motive)
                + "\t" + textureY.getInt(motive));
        }
    }
}
