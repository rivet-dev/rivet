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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGarageOpen.js
var require_faGarageOpen = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGarageOpen.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "garage-open";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e00b";
    var svgPathData = "M0 171.3L0 488c0 13.3 10.7 24 24 24l48 0c13.3 0 24-10.7 24-24l0-264c0-17.7 14.3-32 32-32l384 0c17.7 0 32 14.3 32 32l0 264c0 13.3 10.7 24 24 24l48 0c13.3 0 24-10.7 24-24l0-316.7c0-26.2-15.9-49.7-40.2-59.4L331.9 4.8c-7.6-3.1-16.1-3.1-23.8 0L40.2 111.9C15.9 121.6 0 145.2 0 171.3zM194.1 320l-5.2 15c-17.2 9.5-28.9 27.9-28.9 49l0 96c0 17.7 14.3 32 32 32s32-14.3 32-32l0-16 192 0 0 16c0 17.7 14.3 32 32 32s32-14.3 32-32l0-96c0-21.1-11.7-39.5-28.9-49l-5.2-15L395 320l2.8 8-155.6 0 2.8-8-50.8 0zM240 376a24 24 0 1 1 0 48 24 24 0 1 1 0-48zm136 24a24 24 0 1 1 48 0 24 24 0 1 1 -48 0zM512 224l-384 0 0 64 384 0 0-64z";
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
    exports.faGarageOpen = exports.definition;
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

// src/_entries/faGarageOpen.js
var import_faGarageOpen = __toESM(require_faGarageOpen());
var export_faGarageOpen = import_faGarageOpen.definition;
export {
  export_faGarageOpen as faGarageOpen
};
