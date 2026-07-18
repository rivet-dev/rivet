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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faEarMuffs.js
var require_faEarMuffs = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faEarMuffs.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "ear-muffs";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f795";
    var svgPathData = "M320 64c-78.2 0-143.3 56.1-157.2 130.2c-6-1.4-12.3-2.2-18.8-2.2c-17.7 0-34.1 5.8-47.4 15.6C105 91.5 201.8 0 320 0S535 91.5 543.4 207.6C530.1 197.8 513.7 192 496 192c-6.5 0-12.7 .8-18.8 2.2C463.3 120.1 398.2 64 320 64zM179.8 304c7.6 8.5 12.2 19.7 12.2 32s-4.6 23.5-12.2 32c7.6 8.5 12.2 19.7 12.2 32s-4.6 23.5-12.2 32c7.6 8.5 12.2 19.7 12.2 32c0 26.5-21.5 48-48 48c-22.1 0-40.8-15-46.3-35.4C92.2 478.8 86.2 480 80 480c-26.5 0-48-21.5-48-48c0-6.2 1.2-12.2 3.4-17.7C15 408.8 0 390.1 0 368s15-40.8 35.4-46.3C33.2 316.2 32 310.2 32 304c0-26.5 21.5-48 48-48c6.2 0 12.2 1.2 17.7 3.4C103.2 239 121.9 224 144 224c26.5 0 48 21.5 48 48c0 12.3-4.6 23.5-12.2 32zm280.4 0c-7.6-8.5-12.2-19.7-12.2-32c0-26.5 21.5-48 48-48c22.1 0 40.8 15 46.3 35.4c5.5-2.2 11.4-3.4 17.7-3.4c26.5 0 48 21.5 48 48c0 6.2-1.2 12.2-3.4 17.7C625 327.2 640 345.9 640 368s-15 40.8-35.4 46.3c2.2 5.5 3.4 11.4 3.4 17.7c0 26.5-21.5 48-48 48c-6.2 0-12.2-1.2-17.7-3.4C536.8 497 518.1 512 496 512c-26.5 0-48-21.5-48-48c0-12.3 4.6-23.5 12.2-32c-7.6-8.5-12.2-19.7-12.2-32s4.6-23.5 12.2-32c-7.6-8.5-12.2-19.7-12.2-32s4.6-23.5 12.2-32z";
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
    exports.faEarMuffs = exports.definition;
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

// src/_entries/faEarMuffs.js
var import_faEarMuffs = __toESM(require_faEarMuffs());
var export_faEarMuffs = import_faEarMuffs.definition;
export {
  export_faEarMuffs as faEarMuffs
};
