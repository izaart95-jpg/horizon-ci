import net.minecraft.Bootstrap;
import net.minecraft.item.Item;
import net.minecraft.item.ItemStack;

/**
 * Dump the 1.8.9 name key and English name of every item variant.
 *
 * <p>1.8 names a stack with {@code ItemStack.getTranslationKey() + ".name"};
 * the key varies by damage for the multi-variant items ({@code tile.wool.*},
 * {@code item.swordDiamond}, {@code item.fish.cod.raw}) and the display name
 * adds the entity for a spawn egg. Each line is:
 *
 * <pre>
 * id &lt;tab&gt; damage &lt;tab&gt; translation key &lt;tab&gt; English name
 * </pre>
 *
 * <p>Run it with the 1.8.9 named jar, its libraries and the client jar (for
 * {@code assets/minecraft/lang/en_US.lang}):
 *
 * <pre>
 * javac -cp "$JAR" tools/ItemNameProbe18.java
 * java -cp "$JAR:$CLIENT_JAR:$LIBS:." ItemNameProbe18 &gt; item-names-1.8.9.txt
 * </pre>
 *
 * <p>Then feed the dump to
 * {@code crates/horizon-client/examples/prepare_1_8_9_names.rs}.
 */
public class ItemNameProbe18 {
    public static void main(String[] a) {
        Bootstrap.initialize();
        for (int id = 0; id < 4096; id++) {
            Item item = Item.byRawId(id);
            if (item == null) {
                continue;
            }
            // Every damage a 1.8 variant item uses is below 256: colours,
            // wood types, fish, skulls, banners, spawn eggs. A potion's
            // damage is a bitmask and its name comes from the potion code.
            int damages = item.isDamageable() ? 1 : 256;
            for (int damage = 0; damage < damages; damage++) {
                ItemStack stack = new ItemStack(item, 1, damage);
                String key = stack.getTranslationKey();
                String name;
                try {
                    name = item.getDisplayName(stack);
                } catch (RuntimeException e) {
                    name = "";
                }
                System.out.println(id + "\t" + damage + "\t" + key + "\t" + name);
            }
        }
    }
}
