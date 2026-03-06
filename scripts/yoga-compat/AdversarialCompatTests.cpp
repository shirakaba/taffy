#include <gtest/gtest.h>
#include <yoga/Yoga.h>

// Deliberately antagonistic variations of existing Yoga generated tests.
// Expected values are derived from in-practice Yoga behavior and are intended
// to catch narrow constant-specific compatibility workarounds.

TEST(YogaAdversarialCompatTest, absolute_layout_padding_left_variant) {
  YGConfigRef config = YGConfigNew();

  YGNodeRef root = YGNodeNewWithConfig(config);
  YGNodeStyleSetPositionType(root, YGPositionTypeAbsolute);
  YGNodeStyleSetWidth(root, 320);
  YGNodeStyleSetHeight(root, 180);
  YGNodeStyleSetPadding(root, YGEdgeLeft, 37);
  YGNodeStyleSetPadding(root, YGEdgeRight, 11);

  YGNodeRef child = YGNodeNewWithConfig(config);
  YGNodeStyleSetPositionType(child, YGPositionTypeAbsolute);
  YGNodeStyleSetWidth(child, 61);
  YGNodeStyleSetHeight(child, 23);
  YGNodeInsertChild(root, child, 0);

  YGNodeCalculateLayout(root, YGUndefined, YGUndefined, YGDirectionLTR);
  ASSERT_FLOAT_EQ(37, YGNodeLayoutGetLeft(child));
  ASSERT_FLOAT_EQ(0, YGNodeLayoutGetTop(child));
  ASSERT_FLOAT_EQ(61, YGNodeLayoutGetWidth(child));
  ASSERT_FLOAT_EQ(23, YGNodeLayoutGetHeight(child));

  YGNodeCalculateLayout(root, YGUndefined, YGUndefined, YGDirectionRTL);
  ASSERT_FLOAT_EQ(248, YGNodeLayoutGetLeft(child));
  ASSERT_FLOAT_EQ(0, YGNodeLayoutGetTop(child));
  ASSERT_FLOAT_EQ(61, YGNodeLayoutGetWidth(child));
  ASSERT_FLOAT_EQ(23, YGNodeLayoutGetHeight(child));

  YGNodeFreeRecursive(root);
  YGConfigFree(config);
}

TEST(YogaAdversarialCompatTest, absolute_layout_column_reverse_margin_border_variant) {
  YGConfigRef config = YGConfigNew();

  YGNodeRef root = YGNodeNewWithConfig(config);
  YGNodeStyleSetPositionType(root, YGPositionTypeAbsolute);
  YGNodeStyleSetWidth(root, 310);
  YGNodeStyleSetHeight(root, 180);
  YGNodeStyleSetFlexDirection(root, YGFlexDirectionColumnReverse);

  YGNodeRef child = YGNodeNewWithConfig(config);
  YGNodeStyleSetPositionType(child, YGPositionTypeAbsolute);
  YGNodeStyleSetWidth(child, 70);
  YGNodeStyleSetHeight(child, 40);
  YGNodeStyleSetPosition(child, YGEdgeLeft, 9);
  YGNodeStyleSetPosition(child, YGEdgeRight, 14);
  YGNodeStyleSetMargin(child, YGEdgeLeft, 5);
  YGNodeStyleSetMargin(child, YGEdgeRight, 6);
  YGNodeStyleSetBorder(child, YGEdgeLeft, 2);
  YGNodeStyleSetBorder(child, YGEdgeRight, 11);
  YGNodeInsertChild(root, child, 0);

  YGNodeCalculateLayout(root, YGUndefined, YGUndefined, YGDirectionLTR);
  ASSERT_FLOAT_EQ(14, YGNodeLayoutGetLeft(child));
  ASSERT_FLOAT_EQ(140, YGNodeLayoutGetTop(child));
  ASSERT_FLOAT_EQ(70, YGNodeLayoutGetWidth(child));
  ASSERT_FLOAT_EQ(40, YGNodeLayoutGetHeight(child));

  YGNodeCalculateLayout(root, YGUndefined, YGUndefined, YGDirectionRTL);
  ASSERT_FLOAT_EQ(220, YGNodeLayoutGetLeft(child));
  ASSERT_FLOAT_EQ(140, YGNodeLayoutGetTop(child));
  ASSERT_FLOAT_EQ(70, YGNodeLayoutGetWidth(child));
  ASSERT_FLOAT_EQ(40, YGNodeLayoutGetHeight(child));

  YGNodeFreeRecursive(root);
  YGConfigFree(config);
}

TEST(
    YogaAdversarialCompatTest,
    static_position_absolute_child_insets_relative_to_positioned_ancestor_row_reverse_variant) {
  YGConfigRef config = YGConfigNew();

  YGNodeRef root = YGNodeNewWithConfig(config);
  YGNodeStyleSetPositionType(root, YGPositionTypeAbsolute);

  YGNodeRef level1 = YGNodeNewWithConfig(config);
  YGNodeStyleSetWidth(level1, 300);
  YGNodeStyleSetHeight(level1, 180);
  YGNodeStyleSetFlexDirection(level1, YGFlexDirectionRowReverse);
  YGNodeInsertChild(root, level1, 0);

  YGNodeRef level2 = YGNodeNewWithConfig(config);
  YGNodeStyleSetWidth(level2, 120);
  YGNodeStyleSetHeight(level2, 90);
  YGNodeStyleSetPositionType(level2, YGPositionTypeStatic);
  YGNodeInsertChild(level1, level2, 0);

  YGNodeRef abs_child = YGNodeNewWithConfig(config);
  YGNodeStyleSetWidth(abs_child, 60);
  YGNodeStyleSetHeight(abs_child, 30);
  YGNodeStyleSetPositionType(abs_child, YGPositionTypeAbsolute);
  YGNodeStyleSetPosition(abs_child, YGEdgeTop, 20);
  YGNodeStyleSetPosition(abs_child, YGEdgeLeft, 30);
  YGNodeInsertChild(level2, abs_child, 0);

  YGNodeCalculateLayout(root, YGUndefined, YGUndefined, YGDirectionLTR);
  ASSERT_FLOAT_EQ(-150, YGNodeLayoutGetLeft(abs_child));
  ASSERT_FLOAT_EQ(20, YGNodeLayoutGetTop(abs_child));
  ASSERT_FLOAT_EQ(60, YGNodeLayoutGetWidth(abs_child));
  ASSERT_FLOAT_EQ(30, YGNodeLayoutGetHeight(abs_child));

  YGNodeCalculateLayout(root, YGUndefined, YGUndefined, YGDirectionRTL);
  ASSERT_FLOAT_EQ(30, YGNodeLayoutGetLeft(abs_child));
  ASSERT_FLOAT_EQ(20, YGNodeLayoutGetTop(abs_child));
  ASSERT_FLOAT_EQ(60, YGNodeLayoutGetWidth(abs_child));
  ASSERT_FLOAT_EQ(30, YGNodeLayoutGetHeight(abs_child));

  YGNodeFreeRecursive(root);
  YGConfigFree(config);
}
