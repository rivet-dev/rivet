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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSkiBootSki.js
var require_faSkiBootSki = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSkiBootSki.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "ski-boot-ski";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e3cd";
    var svgPathData = "M390.3 .6c17.3 3.5 28.6 20.3 25.1 37.7l-6.4 32L397.4 128 336 128c-8.8 0-16 7.2-16 16s7.2 16 16 16l55 0-9.6 48L320 208c-8.8 0-16 7.2-16 16s7.2 16 16 16l62.2 0 78.7 34.7c11.6 5.1 19.1 16.6 19.1 29.3l0 80c0 17.7-14.3 32-32 32l-328 0c-9.6 0-18.7-4.3-24.7-11.7s-8.5-17.2-6.6-26.6l24.7-123.6c2.9-.5 6.1-1.2 9.5-2.1c15.6-3.9 36.4-11.6 58.3-26.9c42.4-29.7 86.4-86.4 104.5-193.1l65.7 0 1.3-6.3C356.1 8.4 372.9-2.8 390.3 .6zM162.8 198.9c-15.8 11-30.8 17.2-42.6 20.7L152.6 57.7c3-15 16.1-25.7 31.4-25.7l69.2 0c-17.4 96-56.7 143.3-90.4 166.9zM640 416c0 53-43 96-96 96L32 512c-17.7 0-32-14.3-32-32s14.3-32 32-32l512 0c17.7 0 32-14.3 32-32s14.3-32 32-32s32 14.3 32 32z";
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
    exports.faSkiBootSki = exports.definition;
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

// src/_entries/faSkiBootSki.js
var import_faSkiBootSki = __toESM(require_faSkiBootSki());
var export_faSkiBootSki = import_faSkiBootSki.definition;
export {
  export_faSkiBootSki as faSkiBootSki
};
