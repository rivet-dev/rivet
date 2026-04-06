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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSignalBarsSlash.js
var require_faSignalBarsSlash = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSignalBarsSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "signal-bars-slash";
    var width = 640;
    var height = 512;
    var aliases = ["signal-alt-slash"];
    var unicode = "f694";
    var svgPathData = "M38.8 5.1C28.4-3.1 13.3-1.2 5.1 9.2S-1.2 34.7 9.2 42.9l592 464c10.4 8.2 25.5 6.3 33.7-4.1s6.3-25.5-4.1-33.7L608 451.2 608 48c0-26.5-21.5-48-48-48s-48 21.5-48 48l0 328-64-50.2L448 176c0-26.5-21.5-48-48-48s-48 21.5-48 48l0 74.6L38.8 5.1zM288 322.9l-74.7-58.8C200.5 272.7 192 287.4 192 304l0 160c0 26.5 21.5 48 48 48s48-21.5 48-48l0-141.1zM448 464l0-15-96-75.6 0 90.7c0 26.5 21.5 48 48 48s48-21.5 48-48zM80 384c-26.5 0-48 21.5-48 48l0 32c0 26.5 21.5 48 48 48s48-21.5 48-48l0-32c0-26.5-21.5-48-48-48z";
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
    exports.faSignalBarsSlash = exports.definition;
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

// src/_entries/faSignalAltSlash.js
var import_faSignalBarsSlash = __toESM(require_faSignalBarsSlash());
var export_faSignalAltSlash = import_faSignalBarsSlash.definition;
export {
  export_faSignalAltSlash as faSignalAltSlash
};
