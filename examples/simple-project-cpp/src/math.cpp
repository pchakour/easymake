#include "math.hpp"

Fraction create_half()
{
    Fraction f;
    f.num = 1;  
    f.den = 2;
    return f;
}

Fraction mult(Fraction f1, Fraction f2)
{
    Fraction res;
    res.num = f1.num * f2.num;
    res.den = f1.den * f2.den;
    return res;
}

Fraction invert(Fraction f)
{
    Fraction res;
    res.num = f.den;
    res.den = f.num;
    return res;
}