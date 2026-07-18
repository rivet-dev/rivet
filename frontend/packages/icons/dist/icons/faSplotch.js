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

// src/node_modules/@fortawesome/free-solid-svg-icons/faSplotch.js
var require_faSplotch = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faSplotch.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "splotch";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f5bc";
    var svgPathData = "M208.5 62.3l28.1-36.9C248.8 9.4 267.8 0 288 0 316.5 0 341.6 18.7 349.8 46l17.8 59.4c10.3 34.4 36.1 62 69.8 74.6l39.8 14.9c20.9 7.9 34.8 27.9 34.8 50.2 0 16.9-7.9 32.8-21.5 42.9l-67.3 50.5c-24.3 18.2-37.2 47.9-33.8 78.1l2.5 22.7c4.3 38.7-26 72.6-65 72.6-14.8 0-29.3-5.1-40.9-14.3l-55.3-44.3c-4.5-3.6-9.3-6.7-14.5-9.2-15.8-7.9-33.7-10.4-51-7.3L82.4 451.9c-34.6 6.3-66.4-20.3-66.4-55.4 0-13.2 4.7-26 13.1-36.2l11.2-13.4c14.6-17.4 22.6-39.4 22.6-62.1 0-18.8-5.5-37.2-15.8-53L8.8 173.5C3.1 164.7 0 154.4 0 143.9 0 110.5 30.1 85.1 63 90.7l51.3 8.7c35.9 6.1 72.2-8.2 94.2-37.1z";
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
    exports.faSplotch = exports.definition;
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

// src/_entries/faSplotch.js
var import_faSplotch = __toESM(require_faSplotch());
var export_faSplotch = import_faSplotch.definition;
export {
  export_faSplotch as faSplotch
};
