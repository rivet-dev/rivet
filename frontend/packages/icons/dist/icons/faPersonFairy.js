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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPersonFairy.js
var require_faPersonFairy = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPersonFairy.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "person-fairy";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e608";
    var svgPathData = "M320 0a64 64 0 1 1 0 128A64 64 0 1 1 320 0zM297.6 160l44.8 0c41.6 0 76.3 31.9 79.7 73.4l4.1 49.3c2.5 29.8-15.7 56.1-42.2 65.6L384 464c0 26.5-21.5 48-48 48l-32 0c-26.5 0-48-21.5-48-48l0-115.7c-26.5-9.5-44.7-35.8-42.2-65.6l4.1-49.3c3.5-41.5 38.1-73.4 79.7-73.4zM32 32c89.4 0 169.4 40.8 222.2 104.7c-37.4 15.7-64.6 51.2-68.2 94L181.9 280c-3 36.2 14.4 69 42.1 87.7l0 79.6C191.7 468 153.2 480 112 480l-16 0c-17.7 0-32-14.3-32-32c0-45.7 14.7-88 39.7-122.3C40.6 275.9 0 198.7 0 112L0 64C0 46.3 14.3 32 32 32zM528 480c-41.2 0-79.7-12-112-32.7l0-79.6c27.8-18.7 45.1-51.5 42.1-87.7L454 230.7c-3.6-42.7-30.8-78.2-68.2-94C438.6 72.8 518.6 32 608 32c17.7 0 32 14.3 32 32l0 48c0 86.7-40.6 163.9-103.7 213.7C561.3 360 576 402.3 576 448c0 17.7-14.3 32-32 32l-16 0z";
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
    exports.faPersonFairy = exports.definition;
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

// src/_entries/faPersonFairy.js
var import_faPersonFairy = __toESM(require_faPersonFairy());
var export_faPersonFairy = import_faPersonFairy.definition;
export {
  export_faPersonFairy as faPersonFairy
};
