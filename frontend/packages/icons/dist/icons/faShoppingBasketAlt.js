var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBasketShoppingSimple.js
var require_faBasketShoppingSimple = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBasketShoppingSimple.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "basket-shopping-simple";
    var width = 576;
    var height = 512;
    var aliases = ["shopping-basket-alt"];
    var unicode = "e0af";
    var svgPathData = "M243.1 2.7c11.8 6.1 16.3 20.6 10.2 32.4L171.7 192l232.6 0L322.7 35.1c-6.1-11.8-1.5-26.3 10.2-32.4s26.2-1.5 32.4 10.2L458.4 192l85.6 0c17.7 0 32 14.3 32 32s-14.3 32-32 32L492.1 463.5C485 492 459.4 512 430 512L146 512c-29.4 0-55-20-62.1-48.5L32 256c-17.7 0-32-14.3-32-32s14.3-32 32-32l85.6 0L210.7 12.9c6.1-11.8 20.6-16.3 32.4-10.2zM144 296a24 24 0 1 0 0-48 24 24 0 1 0 0 48zm312-24a24 24 0 1 0 -48 0 24 24 0 1 0 48 0z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faBasketShoppingSimple = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faShoppingBasketAlt.js
var import_faBasketShoppingSimple = __toESM(require_faBasketShoppingSimple());
var export_faShoppingBasketAlt = import_faBasketShoppingSimple.definition;
export {
  export_faShoppingBasketAlt as faShoppingBasketAlt
};
