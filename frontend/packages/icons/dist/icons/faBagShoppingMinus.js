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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBagShoppingMinus.js
var require_faBagShoppingMinus = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBagShoppingMinus.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bag-shopping-minus";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e650";
    var svgPathData = "M224 48c-35.3 0-64 28.7-64 64l0 48 128 0 0-48c0-35.3-28.7-64-64-64zM48 160l64 0 0-48C112 50.1 162.1 0 224 0s112 50.1 112 112l0 48 64 0c26.5 0 48 21.5 48 48l0 208c0 53-43 96-96 96L96 512c-53 0-96-43-96-96L0 208c0-26.5 21.5-48 48-48zm96 152c-13.3 0-24 10.7-24 24s10.7 24 24 24l160 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-160 0z";
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
    exports.faBagShoppingMinus = exports.definition;
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

// src/_entries/faBagShoppingMinus.js
var import_faBagShoppingMinus = __toESM(require_faBagShoppingMinus());
var export_faBagShoppingMinus = import_faBagShoppingMinus.definition;
export {
  export_faBagShoppingMinus as faBagShoppingMinus
};
