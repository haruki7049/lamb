module LambParser.AtomSpec (spec) where

import LambParser.Atoms (Atom (AtomFloat, AtomInteger))
import Test.Hspec (Spec, describe, it, shouldBe, shouldNotBe)

spec :: Spec
spec = do
  describe "Atom" $ do
    it "should be 'AtomInteger 0 == AtomInteger 0'" $
      (AtomInteger 0) `shouldBe` (AtomInteger 0)

    it "should be 'AtomInteger 0 != AtomInteger 1'" $
      (AtomInteger 0) `shouldNotBe` (AtomInteger 1)

    it "should be 'AtomFloat 0.0 == AtomFloat 0.0'" $
      (AtomFloat 0.0) `shouldBe` (AtomFloat 0.0)

    it "should be 'AtomFloat 0.0 != AtomFloat 1.0'" $
      (AtomFloat 0.0) `shouldNotBe` (AtomFloat 1.0)
