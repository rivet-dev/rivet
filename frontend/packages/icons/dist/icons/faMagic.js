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

// src/node_modules/@fortawesome/free-solid-svg-icons/faWandMagic.js
var require_faWandMagic = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faWandMagic.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "wand-magic";
    var width = 512;
    var height = 512;
    var aliases = ["magic"];
    var unicode = "f0d0";
    var svgPathData = "M398.5 12.2l-88.2 88.2 101.3 101.3 88.2-88.2C507.6 105.6 512 95 512 84s-4.4-21.6-12.2-29.5L457.5 12.2C449.6 4.4 439 0 428 0s-21.6 4.4-29.5 12.2zM276.4 134.3L12.2 398.5C4.4 406.4 0 417 0 428s4.4 21.6 12.2 29.5l42.3 42.3C62.4 507.6 73 512 84 512s21.6-4.4 29.5-12.2L377.7 235.6 276.4 134.3z";
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
    exports.faWandMagic = exports.definition;
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

// src/_entries/faMagic.js
var import_faWandMagic = __toESM(require_faWandMagic());
var export_faMagic = import_faWandMagic.definition;
export {
  export_faMagic as faMagic
};
