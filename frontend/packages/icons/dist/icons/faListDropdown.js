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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faListDropdown.js
var require_faListDropdown = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faListDropdown.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "list-dropdown";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e1cf";
    var svgPathData = "M64 32C28.7 32 0 60.7 0 96L0 416c0 35.3 28.7 64 64 64l384 0c35.3 0 64-28.7 64-64l0-320c0-35.3-28.7-64-64-64L64 32zM448 192l0 224L64 416l0-224 384 0zM436.7 96c7.1 0 10.7 8.6 5.7 13.7l-36.7 36.7c-3.1 3.1-8.2 3.1-11.3 0l-36.7-36.7c-5-5-1.5-13.7 5.7-13.7l73.4 0zM128 232c-13.3 0-24 10.7-24 24s10.7 24 24 24l256 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-256 0zm0 96c-13.3 0-24 10.7-24 24s10.7 24 24 24l256 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-256 0z";
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
    exports.faListDropdown = exports.definition;
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

// src/_entries/faListDropdown.js
var import_faListDropdown = __toESM(require_faListDropdown());
var export_faListDropdown = import_faListDropdown.definition;
export {
  export_faListDropdown as faListDropdown
};
