import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import java.util.List;
import net.minecraft.Bootstrap;
import net.minecraft.item.ItemStack;
import net.minecraft.util.WeightedRandomChestContent;

/**
 * Dump 1.8.9's hard-coded chest loot lists.
 *
 * <p>1.8 has no loot table files: every structure holds a static
 * {@code List<WeightedRandomChestContent>} and fills its chests from it. Each
 * line is:
 *
 * <pre>
 * category &lt;tab&gt; item id &lt;tab&gt; damage &lt;tab&gt; min &lt;tab&gt; max &lt;tab&gt; weight &lt;tab&gt; nbt
 * </pre>
 *
 * <p>The category is {@code <class>.<field>}, in list order. The Rust
 * generator maps each category to the current chest table name
 * ({@code crates/horizon-items/src/chest_loot_1_8_9.rs}) and writes the 1.8
 * root's {@code data/minecraft/loot_table/chests/} files.
 *
 * <p>Run it with the 1.8.9 named jar, its libraries and the client jar (for
 * {@code Bootstrap}):
 *
 * <pre>
 * javac -cp "$JAR:$LIBS" tools/ChestLootProbe18.java
 * java -cp "$JAR:$LIBS:." ChestLootProbe18 &gt; chest-loot-1.8.9.txt
 * </pre>
 *
 * <p>Then feed the dump to
 * {@code crates/horizon-client/examples/prepare_1_8_9_chest_loot.rs}.
 */
public class ChestLootProbe18 {
    private static final String[][] SOURCES = {
        {"net.minecraft.world.gen.feature.DungeonFeature", "field_10161"},
        {"net.minecraft.structure.MineshaftPieces", "LOOT_TABLES"},
        {"net.minecraft.structure.VillagePieces$BlacksmithHouse", "POSSIBLE_CHEST_CONTENT"},
        {"net.minecraft.structure.NetherFortressPieces$AbstractPiece", "POSSIBLE_CHEST_CONTENTS"},
        {"net.minecraft.structure.TemplePieces$DesertPyramid", "CHEST_LOOT_TABLE"},
        {"net.minecraft.structure.TemplePieces$JunglePyramid", "CHEST_LOOT_TABLE"},
        {"net.minecraft.structure.TemplePieces$JunglePyramid", "DISPENSER_LOOT_TABLE"},
        {"net.minecraft.structure.StrongholdPieces$Library", "POSSIBLE_CHEST_CONTENT"},
        {"net.minecraft.structure.StrongholdPieces$SquareRoom", "POSSIBLE_CHEST_CONTENT"},
        {"net.minecraft.structure.StrongholdPieces$ChestCorridor", "POSSIBLE_CHEST_CONTENT"},
        {"net.minecraft.server.world.ServerWorld", "BONUS_CHEST_LOOT_TABLE"},
    };

    public static void main(String[] args) throws Exception {
        Bootstrap.initialize();
        for (String[] source : SOURCES) {
            Class<?> owner = Class.forName(source[0]);
            Field field = owner.getDeclaredField(source[1]);
            field.setAccessible(true);
            Object value = field.get(null);
            if (!(value instanceof List<?> list)) {
                throw new IllegalStateException(source[0] + "." + source[1] + " is not a list");
            }
            String category = owner.getSimpleName() + "." + source[1];
            for (Object element : list) {
                if (!(element instanceof WeightedRandomChestContent content)) {
                    throw new IllegalStateException(category + " holds " + element);
                }
                Field itemField = WeightedRandomChestContent.class.getDeclaredField("content");
                Field minField = WeightedRandomChestContent.class.getDeclaredField("minCount");
                Field maxField = WeightedRandomChestContent.class.getDeclaredField("maxCount");
                itemField.setAccessible(true);
                minField.setAccessible(true);
                maxField.setAccessible(true);
                ItemStack stack = (ItemStack) itemField.get(content);
                System.out.println(category + "\t" + net.minecraft.item.Item.getRawId(stack.getItem())
                    + "\t" + stack.getMeta() + "\t" + minField.getInt(content) + "\t"
                    + maxField.getInt(content) + "\t" + weight(content) + "\t"
                    + (stack.hasNbt() ? stack.getNbt().toString() : ""));
            }
        }
    }

    /** `Weighting.Weight.weight`: the field `WeightedRandomChestContent` inherits. */
    private static int weight(WeightedRandomChestContent content) throws Exception {
        Class<?> type = content.getClass();
        while (type != null) {
            for (Field field : type.getDeclaredFields()) {
                if (Modifier.isStatic(field.getModifiers()) || field.getType() != int.class) {
                    continue;
                }
                if (!field.getName().equals("weight")) {
                    continue;
                }
                field.setAccessible(true);
                return field.getInt(content);
            }
            type = type.getSuperclass();
        }
        throw new IllegalStateException("no weight field");
    }
}
