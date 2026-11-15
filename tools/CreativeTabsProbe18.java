import java.util.ArrayList;
import java.util.List;
import java.util.Locale;

import net.minecraft.Bootstrap;
import net.minecraft.enchantment.Enchantment;
import net.minecraft.item.Item;
import net.minecraft.item.ItemStack;
import net.minecraft.item.Items;
import net.minecraft.item.itemgroup.ItemGroup;
import net.minecraft.nbt.NbtCompound;
import net.minecraft.util.Formatting;

/**
 * Dump 1.8.9's creative tabs and their stacks.
 *
 * <p>{@code ItemGroup.showItems} is the client's {@code displayAllRelevantItems}:
 * every registered item whose group is the tab, with the item's own subtype
 * expansion (wool colours, potion damages, spawn eggs, enchanted books...),
 * then the tab's enchantment books. The search tab is built the way
 * {@code CreativeInventoryScreen.search()} builds it: every item with a group,
 * then a book for every enchantment target. Each line is:
 *
 * <pre>
 * G &lt;tab&gt; &lt;raw id&gt; &lt;count&gt; &lt;damage&gt; &lt;display name&gt; &lt;tag or -&gt; &lt;tooltip&gt;
 * I &lt;tab&gt; &lt;raw id&gt; &lt;count&gt; &lt;damage&gt; &lt;display name&gt; &lt;tag or -&gt; &lt;tooltip&gt;
 * S &lt;raw id&gt; &lt;count&gt; &lt;damage&gt; &lt;display name&gt; &lt;tag or -&gt; &lt;tooltip&gt;
 * </pre>
 *
 * <p>{@code G} is a category tab's stack, {@code I} its icon and {@code S} the
 * search list. The tag is the stack's 1.8 {@code tag} compound's SNBT ({@code -}
 * when absent); the numeric id and damage are the 1.8 item form the datafixer
 * turns into the engine's current item and components. The tooltip is
 * {@code ItemStack.getTooltip}'s lines (the ones {@code search()} matches),
 * joined by U+001F.
 *
 * <p>Run it with the 1.8.9 named jar, its libraries and the client jar (for
 * {@code en_US.lang} and the client-only item methods):
 *
 * <pre>
 * javac -cp "$JAR:$CLIENT_JAR:$LIBS" tools/CreativeTabsProbe18.java
 * java -cp "$JAR:$CLIENT_JAR:$LIBS:." CreativeTabsProbe18 &gt; creative-tabs-1.8.9.txt
 * </pre>
 *
 * <p>Then feed the dump to
 * {@code cargo run -p horizon-items --example prepare_1_8_9_creative}.
 */
public class CreativeTabsProbe18 {
    public static void main(String[] args) {
        Bootstrap.initialize();
        for (ItemGroup group : ItemGroup.itemGroups) {
            List<ItemStack> stacks = new ArrayList<>();
            group.showItems(stacks);
            for (ItemStack stack : stacks) {
                System.out.println("G\t" + group.getIndex() + "\t" + line(stack));
            }
            System.out.println("I\t" + group.getIndex() + "\t" + line(group.getIcon()));
        }
        List<ItemStack> search = new ArrayList<>();
        for (Item item : Item.REGISTRY) {
            if (item == null || item.getItemGroup() == null) {
                continue;
            }
            item.appendItemStacks(item, null, search);
        }
        for (Enchantment enchantment : Enchantment.ALL_ENCHANTMENTS) {
            if (enchantment == null || enchantment.target == null) {
                continue;
            }
            Items.ENCHANTED_BOOK.getEnchantments(enchantment, search);
        }
        for (ItemStack stack : search) {
            System.out.println("S\t" + line(stack));
        }
    }

    private static String line(ItemStack stack) {
        String tag = "-";
        NbtCompound full = stack.toNbt(new NbtCompound());
        if (full.contains("tag", 10)) {
            tag = full.getCompound("tag").toString();
        }
        String name = stack.getItem().getDisplayName(stack);
        StringBuilder tooltip = new StringBuilder();
        for (String part : stack.getTooltip(null, false)) {
            if (tooltip.length() > 0) {
                tooltip.append('\u001f');
            }
            // The same text `search()` matches: `Formatting.strip` then lower case.
            tooltip.append(Formatting.strip(part).toLowerCase(Locale.ROOT));
        }
        return Item.getRawId(stack.getItem()) + "\t" + stack.count + "\t" + stack.getDamage()
                + "\t" + name + "\t" + tag + "\t" + tooltip;
    }
}
