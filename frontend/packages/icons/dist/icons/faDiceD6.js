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

// src/node_modules/@fortawesome/free-solid-svg-icons/faDiceD6.js
var require_faDiceD6 = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faDiceD6.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "dice-d6";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f6d1";
    var svgPathData = "M224.4 8.3C244-2.8 268-2.8 287.6 8.3l176 99.7c20 11.4 32.4 32.6 32.4 55.7l0 197.4c0 23-12.4 44.3-32.4 55.7l-176 99.7c-19.6 11.1-43.6 11.1-63.1 0l-176-99.7C28.4 405.5 16 384.2 16 361.2l0-197.4c0-23 12.4-44.3 32.4-55.7l176-99.7zM102.6 155.6c-8.8-3.1-18.8 .3-23.8 8.6s-3.2 18.7 3.6 25l3.2 2.4 150.2 90.2 0 148.7c0 11 9 20 20 20 11 0 20-9 20-20l0-148.7 150.3-90.2c9.5-5.7 12.6-18 6.9-27.4s-18-12.5-27.4-6.9l-149.7 89.8-149.7-89.8-3.7-1.7z";
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
    exports.faDiceD6 = exports.definition;
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

// src/_entries/faDiceD6.js
var import_faDiceD6 = __toESM(require_faDiceD6());
var export_faDiceD6 = import_faDiceD6.definition;
export {
  export_faDiceD6 as faDiceD6
};
