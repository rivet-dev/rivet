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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSolarSystem.js
var require_faSolarSystem = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSolarSystem.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "solar-system";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e02f";
    var svgPathData = "M448 256c0 41.9-13.4 80.6-36.2 112.2C387.3 370.3 368 390.9 368 416c0 26.5 21.5 48 48 48s48-21.5 48-48c0-3.2-.3-6.4-.9-9.4C493.8 364.3 512 312.3 512 256C512 114.6 397.4 0 256 0c-42.3 0-82.1 10.2-117.2 28.4c18.6 11.8 32 31.1 36 53.6C199.4 70.4 227 64 256 64c106 0 192 86 192 192zM256 448C150 448 64 362 64 256c0-41.9 13.4-80.6 36.2-112.2C124.7 141.7 144 121.1 144 96c0-26.5-21.5-48-48-48S48 69.5 48 96c0 3.2 .3 6.4 .9 9.4C18.2 147.7 0 199.7 0 256C0 397.4 114.6 512 256 512c42.1 0 81.7-10.1 116.8-28.1c-18.5-11.9-31.8-31.3-35.6-53.8C312.5 441.6 285 448 256 448zM389.2 182.7c1.8-4.6 2.8-9.5 2.8-14.7c0-22.1-17.9-40-40-40c-4 0-7.8 .6-11.5 1.7C316.4 113.5 287.3 104 256 104c-83.9 0-152 68.1-152 152s68.1 152 152 152s152-68.1 152-152c0-26.6-6.8-51.5-18.8-73.3zM312 168.4c.2 20.6 16 37.6 36.2 39.5c7.5 14.4 11.8 30.8 11.8 48.2c0 57.4-46.6 104-104 104s-104-46.6-104-104s46.6-104 104-104c20.6 0 39.8 6 56 16.4zm8 87.6a64 64 0 1 0 -128 0 64 64 0 1 0 128 0z";
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
    exports.faSolarSystem = exports.definition;
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

// src/_entries/faSolarSystem.js
var import_faSolarSystem = __toESM(require_faSolarSystem());
var export_faSolarSystem = import_faSolarSystem.definition;
export {
  export_faSolarSystem as faSolarSystem
};
