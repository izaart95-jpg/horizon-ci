import net.minecraft.Bootstrap;
import net.minecraft.item.Item;
import net.minecraft.item.ItemStack;
import net.minecraft.recipe.RecipeDispatcher;
import net.minecraft.recipe.RecipeType;
import net.minecraft.recipe.ShapedRecipeType;
import net.minecraft.recipe.ShapelessRecipeType;

import java.lang.reflect.Field;
import java.util.List;

/**
 * Dump the 1.8.9 crafting table's recipes in registration order.
 *
 * <p>1.8 registers its recipes in code, so a JSON dump is not available. Each
 * line is one recipe:
 *
 * <pre>
 * SHAPED &lt;width&gt; &lt;height&gt; &lt;result&gt; &lt;width*height cells&gt;
 * SHAPELESS &lt;result&gt; &lt;ingredients&gt;
 * OTHER &lt;RecipeType class&gt; &lt;result&gt;
 * </pre>
 *
 * <p>A stack is {@code id:count@damage} or {@code -}; damage 32767 is the
 * "any metadata" wildcard. Run it with the 1.8.9 named jar and the libraries
 * the 1.8 launcher profile lists:
 *
 * <pre>
 * javac -cp "$JAR" tools/RecipeProbe18.java
 * java -cp "$JAR:$LIBS:." RecipeProbe18 &gt; recipes18.txt
 * </pre>
 *
 * <p>Then feed the dump to
 * {@code crates/horizon-client/examples/prepare_1_8_9_recipes.rs}.
 */
public class RecipeProbe18 {
    static String stack(ItemStack s) {
        if (s == null) return "-";
        int id = Item.REGISTRY.getRawId(s.getItem());
        String nbt = s.getNbt() == null ? "" : s.getNbt().toString();
        return id + ":" + s.count + "@" + s.getDamage() + (nbt.isEmpty() ? "" : nbt);
    }

    @SuppressWarnings("unchecked")
    public static void main(String[] a) throws Exception {
        Bootstrap.initialize();
        List<RecipeType> recipes = RecipeDispatcher.getInstance().getAllRecipes();
        for (RecipeType r : recipes) {
            if (r instanceof ShapedRecipeType) {
                Field w = ShapedRecipeType.class.getDeclaredField("field_4438");
                Field h = ShapedRecipeType.class.getDeclaredField("field_4439");
                Field in = ShapedRecipeType.class.getDeclaredField("field_4440");
                w.setAccessible(true);
                h.setAccessible(true);
                in.setAccessible(true);
                ItemStack[] stacks = (ItemStack[]) in.get(r);
                StringBuilder sb = new StringBuilder("SHAPED ");
                sb.append(w.getInt(r)).append(' ').append(h.getInt(r)).append(' ').append(stack(r.getOutput()));
                for (ItemStack st : stacks) sb.append(' ').append(stack(st));
                System.out.println(sb);
            } else if (r instanceof ShapelessRecipeType) {
                Field in = ShapelessRecipeType.class.getDeclaredField("stacks");
                in.setAccessible(true);
                List<ItemStack> stacks = (List<ItemStack>) in.get(r);
                StringBuilder sb = new StringBuilder("SHAPELESS ").append(stack(r.getOutput()));
                for (ItemStack st : stacks) sb.append(' ').append(stack(st));
                System.out.println(sb);
            } else {
                System.out.println("OTHER " + r.getClass().getSimpleName() + " " + stack(r.getOutput()));
            }
        }
    }
}
