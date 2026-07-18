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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCroissant.js
var require_faCroissant = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCroissant.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "croissant";
    var width = 512;
    var height = 512;
    var aliases = [129360];
    var unicode = "f7f6";
    var svgPathData = "M259.4 1.4c3.9 5.9 7 12.5 9.1 19.5l59.9 199.7c1.2 4.1 2 8.2 2.4 12.4L440.3 92.2c13.6-17.4 10.5-42.8-8.6-53.9C389.4 14 340.3 0 288 0c-9.7 0-19.2 .5-28.6 1.4zM233 330.8c-4.1-.4-8.3-1.2-12.4-2.4L20.9 268.5c-7-2.1-13.6-5.2-19.5-9.1C.5 268.8 0 278.3 0 288c0 52.3 14 101.4 38.4 143.7c11 19.1 36.4 22.2 53.9 8.6L233 330.8zM232.7 19s0 0 0 0c2 3 3.6 6.4 4.8 10c-1.2-3.6-2.8-6.9-4.8-10c0 0 0 0 0 0zm232.9 92.8L390 209l87.6 14.6c1.6 .3 3.2 .4 4.9 .4c16.3 0 29.6-13.2 29.6-29.6l0-5.4c0-4.2-.7-8.3-2.3-12.2c-10.1-25.2-24.1-48.5-41.3-69c-.9 1.4-1.9 2.7-2.9 4zM209 390l-97.1 75.6c-1.3 1-2.6 2-4 2.9c20.5 17.2 43.8 31.2 69 41.3c3.9 1.5 8 2.3 12.2 2.3l5.4 0c16.3 0 29.6-13.2 29.6-29.6c0-1.6-.1-3.3-.4-4.9L209 390zM19 232.7s0 0 0 0c2.3 1.5 4.7 2.8 7.3 3.8c1.2 .5 2.5 1 3.8 1.3l199.7 59.9c11.3 3.4 23.5 .3 31.8-8l28.1-28.1c8.3-8.3 11.4-20.5 8-31.8L237.8 30.1C232.5 12.2 216 0 197.4 0c-4.6 0-9.2 .7-13.5 2.3C99.7 33 33 99.7 2.3 183.9C.7 188.2 0 192.8 0 197.4c0 14.5 7.4 27.6 19 35.3c0 0 0 0 0 0z";
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
    exports.faCroissant = exports.definition;
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

// src/_entries/faCroissant.js
var import_faCroissant = __toESM(require_faCroissant());
var export_faCroissant = import_faCroissant.definition;
export {
  export_faCroissant as faCroissant
};
