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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCandleHolder.js
var require_faCandleHolder = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCandleHolder.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "candle-holder";
    var width = 448;
    var height = 512;
    var aliases = [128367];
    var unicode = "f6bc";
    var svgPathData = "M160 192c-44.2 0-80-35.8-80-80c0-39.6 49.1-90.1 66.2-106.6C149.9 1.9 154.8 0 160 0s10.1 1.9 13.8 5.4C190.9 21.9 240 72.4 240 112c0 44.2-35.8 80-80 80zm-32 32l0 48c0 8.8 7.2 16 16 16s16-7.2 16-16l0-48 64 0c17.7 0 32 14.3 32 32l0 208 52.1 0c-2.7-7.5-4.1-15.6-4.1-24c0-39.8 32.2-72 72-72s72 32.2 72 72s-32.2 72-72 72L24 512c-13.3 0-24-10.7-24-24s10.7-24 24-24l40 0 0-208c0-17.7 14.3-32 32-32l32 0zM400 440a24 24 0 1 0 -48 0 24 24 0 1 0 48 0z";
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
    exports.faCandleHolder = exports.definition;
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

// src/_entries/faCandleHolder.js
var import_faCandleHolder = __toESM(require_faCandleHolder());
var export_faCandleHolder = import_faCandleHolder.definition;
export {
  export_faCandleHolder as faCandleHolder
};
