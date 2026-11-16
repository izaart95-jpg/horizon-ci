import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;
import java.util.Random;

/**
 * Dump 1.8.9's enchantment tables from the real server jar.
 *
 * <p>The 1.8 jar is obfuscated, so this probe lives in the default package and
 * names the classes the way the jar does: {@code aci} is {@code Enchantment},
 * {@code ack} is {@code EnchantmentHelper}, {@code zw} is {@code Item},
 * {@code zx} is {@code ItemStack}, {@code acj} is {@code EnumEnchantmentType},
 * {@code kb} is {@code Bootstrap}, {@code jy} is {@code ResourceLocation}.
 *
 * <p>Lines:
 *
 * <pre>
 * E &lt;id&gt; &lt;name&gt; &lt;weight&gt; &lt;min level&gt; &lt;max level&gt; &lt;type ordinal&gt; &lt;first1&gt;..&lt;first5&gt; &lt;last1&gt;..&lt;last5&gt; &lt;compatible ids&gt;
 * A &lt;name&gt; &lt;mask&gt;
 * I &lt;name&gt; &lt;enchantability&gt; &lt;type mask&gt;
 * T &lt;item&gt; &lt;level&gt; &lt;seed&gt; &lt;id:level,...&gt; &lt;nextInt1000&gt;
 * </pre>
 *
 * <p>In an {@code A} line bit {@code n} is set when the {@code n}-th
 * enchantment in ascending id order (the {@code E} lines' order) accepts the
 * item, which is {@code Enchantment.canApply} including the overrides.
 * {@code type mask} has bit {@code n} set when {@code acj.values()[n]}
 * accepts the item, in the enum order ALL, ARMOR, ARMOR_FEET, ARMOR_LEGS,
 * ARMOR_CHEST, ARMOR_HEAD, WEAPON, DIGGING, FISHING_ROD, BREAKABLE, BOW.
 *
 * <p>Run it with the 1.8.9 server jar:
 *
 * <pre>
 * javac -cp .local/minecraft/1.8.9/server.jar -d /tmp/ench18 tools/EnchantProbe18.java
 * java -cp .local/minecraft/1.8.9/server.jar:/tmp/ench18 EnchantProbe18 \
 *     &gt; .local/minecraft/1.8.9/enchant-1.8.9.txt
 * </pre>
 */
public class EnchantProbe18 {
    public static void main(String[] args) {
        kb.c();
        List<jy> names = new ArrayList<>(aci.c());
        // Registry order is HashMap order on names; sort by the numeric id so
        // the dump reads in ascending 1.8 id, as `aci.b` does.
        names.sort(Comparator.comparingInt(n -> aci.b(n.toString()).B));

        // -- enchantments --------------------------------------------------
        for (jy name : names) {
            aci e = aci.b(name.toString());
            StringBuilder line = new StringBuilder();
            line.append("E ").append(e.B).append(' ').append(name);
            line.append(' ').append(e.d());
            line.append(' ').append(e.e()).append(' ').append(e.b());
            // `buildEnchantmentList` gates candidates on the enchantment's
            // `EnumEnchantmentType`, not on `Enchantment.canApply`.
            line.append(' ').append(e.C.ordinal());
            for (int lvl = 1; lvl <= 5; lvl++) {
                line.append(' ').append(e.a(lvl));
            }
            for (int lvl = 1; lvl <= 5; lvl++) {
                line.append(' ').append(e.b(lvl));
            }
            for (jy other : names) {
                aci o = aci.b(other.toString());
                if (e.a(o)) {
                    line.append(' ').append(o.B);
                }
            }
            System.out.println(line);
        }

        // -- what each item accepts ----------------------------------------
        // `Enchantment.canApply` per item, so the overrides (`acf`'s sword
        // check, the looting/fortune types) need no mirroring here.
        acj[] types = acj.values();
        for (zw item : zw.e) {
            jy name = zw.e.c(item);
            if (name == null) {
                continue;
            }
            int accepts = 0;
            zx stack = new zx(item);
            for (int i = 0; i < names.size(); i++) {
                if (aci.b(names.get(i).toString()).a(stack)) {
                    accepts |= 1 << i;
                }
            }
            System.out.println("A " + name + " " + accepts);
        }

        // -- items ---------------------------------------------------------
        for (zw item : zw.e) {
            jy name = zw.e.c(item);
            if (name == null) {
                continue;
            }
            int mask = 0;
            for (int i = 0; i < types.length; i++) {
                if (types[i].a(item)) {
                    mask |= 1 << i;
                }
            }
            System.out.println("I " + name + " " + item.b() + " " + mask);
        }

        // -- random builds -------------------------------------------------
        // Every item with an enchantability, the equipment a 1.8 monster
        // spawns with included, at the levels `addRandomEnchantment` gets.
        int[] levels = {1, 5, 9, 14, 19, 22, 26, 30, 39};
        long[] seeds = {0L, 42L};
        for (zw item : zw.e) {
            jy name = zw.e.c(item);
            if (name == null || item.b() <= 0) {
                continue;
            }
            for (int level : levels) {
                for (long seed : seeds) {
                    Random random = new Random(seed);
                    List<acl> list = ack.b(random, new zx(item), level);
                    StringBuilder line = new StringBuilder();
                    line.append("T ").append(name).append(' ').append(level);
                    line.append(' ').append(seed).append(' ');
                    if (list == null) {
                        line.append('-');
                    } else {
                        for (int i = 0; i < list.size(); i++) {
                            if (i > 0) {
                                line.append(',');
                            }
                            acl e = list.get(i);
                            line.append(e.b.B).append(':').append(e.c);
                        }
                    }
                    line.append(' ').append(random.nextInt(1000));
                    System.out.println(line);
                }
            }
        }
    }
}
