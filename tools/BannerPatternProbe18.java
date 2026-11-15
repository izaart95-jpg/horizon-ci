import net.minecraft.block.entity.BannerBlockEntity;

/**
 * Dump 1.8.9's banner patterns.
 *
 * <p>1.8 has no {@code banner_pattern} registry: {@code BannerBlockEntity.BannerPattern}
 * is an enum of display name, two-letter save code, and (for the craftable
 * patterns) the ingredient stack. A saved banner layer's {@code Pattern} is
 * that code, while the texture and the language key use the display name:
 * {@code textures/entity/banner/<name>.png} and {@code item.banner.<name>.<colour>}.
 * Each line is:
 *
 * <pre>
 * enumName &lt;tab&gt; saveCode &lt;tab&gt; name &lt;tab&gt; hasIngredient &lt;tab&gt; ingredientItem@damage
 * </pre>
 *
 * <p>The Rust generator checks {@code horizon_items::banner_patterns_1_8_9} and
 * the data fixer's code table against this dump, then writes the
 * {@code data/minecraft/banner_pattern/*.json} files.
 *
 * <pre>
 * javac -cp "$JAR" -d /tmp/opencode tools/BannerPatternProbe18.java
 * java -cp "$JAR:/tmp/opencode" BannerPatternProbe18 &gt; banners-1.8.9.txt
 * </pre>
 */
public class BannerPatternProbe18 {
    public static void main(String[] args) throws Exception {
        net.minecraft.Bootstrap.initialize();
        Class<?> patterns = Class.forName(
            "net.minecraft.block.entity.BannerBlockEntity$BannerPattern");
        for (Object pattern : patterns.getEnumConstants()) {
            String enumName = ((Enum<?>) pattern).name();
            String id = (String) patterns.getMethod("getId").invoke(pattern);
            String name = (String) patterns.getMethod("getName").invoke(pattern);
            boolean hasIngredient = (Boolean) patterns.getMethod("hasIngredient").invoke(pattern);
            net.minecraft.item.ItemStack ingredient =
                (net.minecraft.item.ItemStack) patterns.getMethod("getIngredient").invoke(pattern);
            String item = ingredient == null ? "-" : net.minecraft.item.Item.REGISTRY
                .getIdentifier(ingredient.getItem()) + "@" + ingredient.getDamage();
            System.out.println(enumName + "\t" + id + "\t" + name + "\t" + hasIngredient + "\t" + item);
        }
    }
}
