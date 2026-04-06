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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCoffeePot.js
var require_faCoffeePot = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCoffeePot.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "coffee-pot";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e002";
    var svgPathData = "M64 128c0-17.7 14.3-32 32-32l12.8 0 34 84.9C95 215.9 64 272.3 64 336l0 8c0 55.5 34.6 99.4 55.9 121.2c9.9 10.2 23.9 14.8 38.1 14.8L418 480c14.2 0 28.1-4.7 38.1-14.8C477.4 443.4 512 399.5 512 344l0-8c0-63.7-31-120.1-78.8-155.1L478.1 68.8c1.3-3.2 1.9-6.6 1.9-10C480 44 468 32 453.2 32L192 32l-69.2 0L96 32C43 32 0 75 0 128l0 64c0 17.7 14.3 32 32 32s32-14.3 32-32l0-64zm65 192c5.2-41.4 30.1-76.6 65-96l188 0c34.9 19.4 59.8 54.6 65 96l-318 0z";
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
    exports.faCoffeePot = exports.definition;
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

// src/_entries/faCoffeePot.js
var import_faCoffeePot = __toESM(require_faCoffeePot());
var export_faCoffeePot = import_faCoffeePot.definition;
export {
  export_faCoffeePot as faCoffeePot
};
