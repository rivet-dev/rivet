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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faOilCanDrip.js
var require_faOilCanDrip = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faOilCanDrip.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "oil-can-drip";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e205";
    var svgPathData = "M352 96c0 17.7-14.3 32-32 32l-32 0 0 32 80.9 0c9.9 0 19.7 2.3 28.6 6.8L448 192l167-35.8c18.1-3.9 30.7 17.6 18.4 31.5L451.1 394.3c-12.1 13.8-29.6 21.7-48 21.7L144 416c-26.5 0-48-21.5-48-48l0-21.3-67.5-30C11.2 309 0 291.8 0 272.8L0 208c0-26.5 21.5-48 48-48l48 0 48 0 80 0 0-32-32 0c-17.7 0-32-14.3-32-32s14.3-32 32-32l128 0c17.7 0 32 14.3 32 32zM48 208l0 64.8 48 21.3L96 208l-48 0zM592 448c-26.5 0-48-21.5-48-48c0-21 20.6-62.6 34.9-88.8c5.8-10.6 20.5-10.6 26.3 0C619.4 337.4 640 379 640 400c0 32-21.5 48-48 48z";
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
    exports.faOilCanDrip = exports.definition;
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

// src/_entries/faOilCanDrip.js
var import_faOilCanDrip = __toESM(require_faOilCanDrip());
var export_faOilCanDrip = import_faOilCanDrip.definition;
export {
  export_faOilCanDrip as faOilCanDrip
};
