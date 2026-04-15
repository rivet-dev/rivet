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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGrillFire.js
var require_faGrillFire = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGrillFire.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "grill-fire";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e5a4";
    var svgPathData = "M0 72C0 49.9 17.9 32 40 32l368 0c22.1 0 40 17.9 40 40l0 24c0 24.1-3.8 47.3-10.9 69.1c-8.5-8.6-17.2-17-26.2-25.1c-17.7-16-44.7-16-62.5-.1c-35.4 31.6-65.6 67.7-87.3 102.8c-15 24.3-27.7 51-33.6 77.3c-1.2 0-2.4 0-3.6 0c-16.9 0-33.4-1.9-49.3-5.4L151.8 368l73 0c1.5 16.6 5.2 32.7 10.7 48L160 416c0 35.3-28.7 64-64 64s-64-28.7-64-64s28.7-64 64-64c3.4 0 6.8 .3 10.1 .8L129.2 299C52.9 263.3 0 185.8 0 96L0 72zM96 440a24 24 0 1 0 0-48 24 24 0 1 0 0 48zm320 72c-88.4 0-160-71.6-160-160c0-76.7 62.5-144.7 107.2-179.4c5-3.9 10.9-5.8 16.8-5.8c7.9-.1 16 3.1 22 9.2l46 46 11.3-11.3c11.7-11.7 30.6-12.7 42.3-1C560.5 268 576 320.2 576 352c0 88.4-71.6 160-160 160zm64-111.8c0-36.5-37-73-54.8-88.4c-5.4-4.7-13.1-4.7-18.5 0C389 327.1 352 363.6 352 400.2c0 35.3 28.7 64 64 64s64-28.7 64-64z";
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
    exports.faGrillFire = exports.definition;
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

// src/_entries/faGrillFire.js
var import_faGrillFire = __toESM(require_faGrillFire());
var export_faGrillFire = import_faGrillFire.definition;
export {
  export_faGrillFire as faGrillFire
};
