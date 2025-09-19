#pragma once

struct Fraction
{
    int num;
    int den;
}; 

Fraction create_half();
Fraction mult(Fraction f1, Fraction f2);
Fraction invert(Fraction f);