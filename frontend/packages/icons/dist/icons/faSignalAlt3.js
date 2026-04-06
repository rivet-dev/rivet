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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSignalBarsGood.js
var require_faSignalBarsGood = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSignalBarsGood.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "signal-bars-good";
    var width = 640;
    var height = 512;
    var aliases = ["signal-alt-3"];
    var unicode = "f693";
    var svgPathData = "M400 128c-26.5 0-48 21.5-48 48l0 288c0 26.5 21.5 48 48 48s48-21.5 48-48l0-288c0-26.5-21.5-48-48-48zM192 304l0 160c0 26.5 21.5 48 48 48s48-21.5 48-48l0-160c0-26.5-21.5-48-48-48s-48 21.5-48 48zM32 432l0 32c0 26.5 21.5 48 48 48s48-21.5 48-48l0-32c0-26.5-21.5-48-48-48s-48 21.5-48 48z";
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
    exports.faSignalBarsGood = exports.definition;
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

// src/_entries/faSignalAlt3.js
var import_faSignalBarsGood = __toESM(require_faSignalBarsGood());
var export_faSignalAlt3 = import_faSignalBarsGood.definition;
export {
  export_faSignalAlt3 as faSignalAlt3
};
