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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCompassSlash.js
var require_faCompassSlash = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCompassSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "compass-slash";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f5e9";
    var svgPathData = "M38.8 5.1C28.4-3.1 13.3-1.2 5.1 9.2S-1.2 34.7 9.2 42.9l592 464c10.4 8.2 25.5 6.3 33.7-4.1s6.3-25.5-4.1-33.7l-95.4-74.8C561.1 354.5 576 307 576 256C576 114.6 461.4 0 320 0C246.8 0 180.9 30.7 134.2 79.9L38.8 5.1zM270.3 186.5l143.3-55.1c19.4-7.5 38.5 11.6 31 31L397.1 285.9 270.3 186.5zM320 512c55.2 0 106.3-17.5 148.1-47.2L317 345.8l-90.6 34.8c-19.4 7.5-38.5-11.6-31-31l29.4-76.5L82.3 160.8C70.5 190.2 64 222.4 64 256c0 141.4 114.6 256 256 256z";
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
    exports.faCompassSlash = exports.definition;
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

// src/_entries/faCompassSlash.js
var import_faCompassSlash = __toESM(require_faCompassSlash());
var export_faCompassSlash = import_faCompassSlash.definition;
export {
  export_faCompassSlash as faCompassSlash
};
