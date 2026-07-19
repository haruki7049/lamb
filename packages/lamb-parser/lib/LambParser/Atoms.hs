module LambParser.Atoms (Atom (AtomInteger, AtomFloat, AtomString)) where

data Atom a where
  AtomString :: String -> Atom String
  AtomInteger :: Int -> Atom Integer
  AtomFloat :: Float -> Atom Float

deriving instance Show (Atom a)

deriving instance Eq (Atom a)
