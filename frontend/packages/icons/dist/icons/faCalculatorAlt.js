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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCalculatorSimple.js
var require_faCalculatorSimple = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCalculatorSimple.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "calculator-simple";
    var width = 512;
    var height = 512;
    var aliases = ["calculator-alt"];
    var unicode = "f64c";
    var svgPathData = "M0 48C0 21.5 21.5 0 48 0L176 0c26.5 0 48 21.5 48 48l0 128c0 26.5-21.5 48-48 48L48 224c-26.5 0-48-21.5-48-48L0 48zm288 0c0-26.5 21.5-48 48-48L464 0c26.5 0 48 21.5 48 48l0 128c0 26.5-21.5 48-48 48l-128 0c-26.5 0-48-21.5-48-48l0-128zm48 240l128 0c26.5 0 48 21.5 48 48l0 128c0 26.5-21.5 48-48 48l-128 0c-26.5 0-48-21.5-48-48l0-128c0-26.5 21.5-48 48-48zM0 336c0-26.5 21.5-48 48-48l128 0c26.5 0 48 21.5 48 48l0 128c0 26.5-21.5 48-48 48L48 512c-26.5 0-48-21.5-48-48L0 336zM416 64c0-8.8-7.2-16-16-16s-16 7.2-16 16l0 32-32 0c-8.8 0-16 7.2-16 16s7.2 16 16 16l32 0 0 32c0 8.8 7.2 16 16 16s16-7.2 16-16l0-32 32 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-32 0 0-32zM64 96c-8.8 0-16 7.2-16 16s7.2 16 16 16l96 0c8.8 0 16-7.2 16-16s-7.2-16-16-16L64 96zM83.3 348.7c-6.2-6.2-16.4-6.2-22.6 0s-6.2 16.4 0 22.6L89.4 400 60.7 428.7c-6.2 6.2-6.2 16.4 0 22.6s16.4 6.2 22.6 0L112 422.6l28.7 28.7c6.2 6.2 16.4 6.2 22.6 0s6.2-16.4 0-22.6L134.6 400l28.7-28.7c6.2-6.2 6.2-16.4 0-22.6s-16.4-6.2-22.6 0L112 377.4 83.3 348.7zM352 352c-8.8 0-16 7.2-16 16s7.2 16 16 16l96 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-96 0zm0 64c-8.8 0-16 7.2-16 16s7.2 16 16 16l96 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-96 0z";
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
    exports.faCalculatorSimple = exports.definition;
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

// src/_entries/faCalculatorAlt.js
var import_faCalculatorSimple = __toESM(require_faCalculatorSimple());
var export_faCalculatorAlt = import_faCalculatorSimple.definition;
export {
  export_faCalculatorAlt as faCalculatorAlt
};
