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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPersonDressFairy.js
var require_faPersonDressFairy = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPersonDressFairy.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "person-dress-fairy";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e607";
    var svgPathData = "M320 0a64 64 0 1 1 0 128A64 64 0 1 1 320 0zM304.3 160l31.5 0c28.6 0 53.7 18.9 61.5 46.4l39.1 136.8c5.8 20.4-9.5 40.8-30.8 40.8L384 384l0 80c0 26.5-21.5 48-48 48l-32 0c-26.5 0-48-21.5-48-48l0-80-21.6 0c-21.3 0-36.6-20.3-30.8-40.8l39.1-136.8c7.8-27.5 33-46.4 61.5-46.4zM32 32c91.1 0 172.4 42.3 225.1 108.4c-21.5 12.1-38 32.3-45.2 57.3L172.9 334.4c-10.7 37.5 14.2 74.8 51.1 80.7l0 32.1C191.7 468 153.2 480 112 480l-16 0c-17.7 0-32-14.3-32-32c0-45.7 14.7-88 39.7-122.3C40.6 275.9 0 198.7 0 112L0 64C0 46.3 14.3 32 32 32zM528 480c-41.2 0-79.7-12-112-32.7l0-32.1c36.9-6 61.8-43.3 51.1-80.7L428 197.6c-7.1-24.9-23.7-45.1-45.2-57.3C435.6 74.3 516.9 32 608 32c17.7 0 32 14.3 32 32l0 48c0 86.7-40.6 163.9-103.7 213.7C561.3 360 576 402.3 576 448c0 17.7-14.3 32-32 32l-16 0z";
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
    exports.faPersonDressFairy = exports.definition;
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

// src/_entries/faPersonDressFairy.js
var import_faPersonDressFairy = __toESM(require_faPersonDressFairy());
var export_faPersonDressFairy = import_faPersonDressFairy.definition;
export {
  export_faPersonDressFairy as faPersonDressFairy
};
