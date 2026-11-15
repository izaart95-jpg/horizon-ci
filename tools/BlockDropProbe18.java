import net.minecraft.Bootstrap;
import net.minecraft.block.Block;
import net.minecraft.block.BlockState;
import net.minecraft.item.Item;
import net.minecraft.item.ItemStack;
import net.minecraft.state.StateManager;

import java.lang.reflect.Method;
import java.util.ArrayList;
import java.util.List;
import java.util.Random;

/**
 * Dump the 1.8.9 drop behaviour of every block state.
 *
 * <p>The probe calls the same methods {@code Block.randomDropAsItem} uses,
 * with a seeded {@link Random}, so the Rust side can see which blocks differ
 * from the default "one of the block's own item" rule:
 *
 * <pre>
 * BLOCK &lt;id&gt; &lt;name&gt; &lt;class&gt;
 * STATE &lt;id&gt; &lt;meta&gt; &lt;props&gt; drop=&lt;item&gt;@&lt;meta&gt; count=&lt;n&gt;
 *       bonus=&lt;f0,f1,f2,f3&gt; silk=&lt;item&gt;@&lt;meta&gt;
 *       harvest=&lt;class&gt; random=&lt;class&gt; silkclass=&lt;class&gt;
 * </pre>
 *
 * <p>Props are {@code name=value} pairs separated by commas. Run it with the
 * 1.8.9 named jar and the libraries the launcher profile lists:
 *
 * <pre>
 * javac -cp "$JAR:$LIBS" tools/BlockDropProbe18.java
 * java -cp "$JAR:$LIBS:." BlockDropProbe18 &gt; block-drops18.txt
 * </pre>
 */
public class BlockDropProbe18 {
    static String item(Item item) {
        if (item == null) return "-";
        return Integer.toString(Item.REGISTRY.getRawId(item));
    }

    static String stack(ItemStack s) {
        if (s == null) return "-";
        return item(s.getItem()) + "@" + s.getDamage();
    }

    static String declaring(Block block, String method, Class<?>... args) {
        try {
            Method m = block.getClass().getMethod(method, args);
            return m.getDeclaringClass().getName();
        } catch (NoSuchMethodException e) {
            return "-";
        }
    }

    static String declaringProtected(Block block, String method) {
        try {
            Method m = block.getClass().getDeclaredMethod(method);
            m.setAccessible(true);
            return m.getDeclaringClass().getName();
        } catch (NoSuchMethodException e) {
            Class<?> c = block.getClass();
            while (c != null) {
                try {
                    Method m = c.getDeclaredMethod(method);
                    m.setAccessible(true);
                    return m.getDeclaringClass().getName();
                } catch (NoSuchMethodException ignored) {
                    c = c.getSuperclass();
                }
            }
            return "-";
        }
    }

    public static void main(String[] args) throws Exception {
        Bootstrap.initialize();
        for (Block block : Block.REGISTRY) {
            String name = Block.REGISTRY.getIdentifier(block).toString();
            int id = Block.getIdByBlock(block);
            System.out.println("BLOCK " + id + " " + name + " " + block.getClass().getName());
            StateManager manager = block.getStateManager();
            for (BlockState state : manager.getBlockStates()) {
                int meta = block.getData(state);
                int count = block.getDropCount(new Random(0x5eedL));
                int[] bonus = new int[4];
                for (int f = 0; f < 4; f++) {
                    bonus[f] = block.getBonusDrops(f, new Random(0x5eedL + f));
                }
                ItemStack silk;
                try {
                    Method m = Block.class.getDeclaredMethod("createStackFromBlock", BlockState.class);
                    m.setAccessible(true);
                    silk = (ItemStack) m.invoke(block, state);
                } catch (Exception e) {
                    silk = null;
                }
                boolean silkTouch;
                try {
                    Method m = Block.class.getDeclaredMethod("requiresSilkTouch");
                    m.setAccessible(true);
                    silkTouch = (Boolean) m.invoke(block);
                } catch (Exception e) {
                    silkTouch = false;
                }
                System.out.println("STATE " + id + " " + meta
                        + " drop=" + item(block.getDropItem(state, new Random(0x5eedL), 0))
                        + " count=" + count
                        + " bonus=" + bonus[0] + "," + bonus[1] + "," + bonus[2] + "," + bonus[3]
                        + " silk=" + stack(silk)
                        + " silkrequired=" + silkTouch
                        + " metaout=" + block.getMeta(state)
                        + " harvest=" + declaring(block, "harvest",
                                net.minecraft.world.World.class,
                                net.minecraft.entity.player.PlayerEntity.class,
                                net.minecraft.util.math.BlockPos.class,
                                BlockState.class,
                                net.minecraft.block.entity.BlockEntity.class)
                        + " random=" + declaring(block, "randomDropAsItem",
                                net.minecraft.world.World.class,
                                net.minecraft.util.math.BlockPos.class,
                                BlockState.class,
                                float.class,
                                int.class)
                        + " bonusclass=" + declaring(block, "getBonusDrops", int.class, Random.class)
                        + " itemclass=" + declaring(block, "getDropItem",
                                BlockState.class, Random.class, int.class)
                        + " countclass=" + declaring(block, "getDropCount", Random.class)
                        + " metaclass=" + declaring(block, "getMeta", BlockState.class));
            }
        }
    }
}
