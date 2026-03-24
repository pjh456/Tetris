export const Actions = {
  MoveLeft: 0,
  MoveRight: 1,
  SoftDrop: 2,
  HardDrop: 3,
  RotateCW: 4,
  RotateCCW: 5,
  Hold: 6
} as const;

export type ActionValue = (typeof Actions)[keyof typeof Actions];
