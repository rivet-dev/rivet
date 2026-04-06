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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCauldron.js
var require_faCauldron = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCauldron.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "cauldron";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f6bf";
    var svgPathData = "M160 64a32 32 0 1 0 0-64 32 32 0 1 0 0 64zm112 64a48 48 0 1 0 0-96 48 48 0 1 0 0 96zM35.1 224C16.5 257 0 297.1 0 334.4c0 37.4 11.9 69.4 32 95.5L32 488c0 13.3 10.7 24 24 24s24-10.7 24-24l0-15c40.4 25.5 91.8 39 144 39s103.6-13.5 144-39l0 15c0 13.3 10.7 24 24 24s24-10.7 24-24l0-58.1c20.1-26.1 32-58.1 32-95.5c0-37.3-16.5-77.4-35.1-110.4l3.1 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-48 0-144 0L80 160l-48 0c-17.7 0-32 14.3-32 32s14.3 32 32 32l3.1 0z";
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
    exports.faCauldron = exports.definition;
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

// src/_entries/faCauldron.js
var import_faCauldron = __toESM(require_faCauldron());
var export_faCauldron = import_faCauldron.definition;
export {
  export_faCauldron as faCauldron
};
