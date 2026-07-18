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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTreeDecorated.js
var require_faTreeDecorated = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTreeDecorated.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "tree-decorated";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f7dc";
    var svgPathData = "M210.6 5.9L62 169.4c-3.9 4.2-6 9.8-6 15.5C56 197.7 66.3 208 79.1 208l24.9 0L28.5 313.7c-2.9 4.1-4.5 9-4.5 14.1C24 341.1 34.8 352 48.2 352L80 352 3.8 474c-2.5 4-3.8 8.5-3.8 13.2C0 500.9 11.1 512 24.9 512l398.3 0c13.7 0 24.9-11.1 24.9-24.9c0-4.7-1.3-9.2-3.8-13.2L368 352l31.8 0c13.4 0 24.2-10.9 24.2-24.2c0-5-1.6-10-4.5-14.1L344 208l24.9 0c12.7 0 23.1-10.3 23.1-23.1c0-5.7-2.1-11.3-6-15.5L237.4 5.9C234 2.1 229.1 0 224 0s-10 2.1-13.4 5.9zM160 264a24 24 0 1 1 0 48 24 24 0 1 1 0-48zm72-104a24 24 0 1 1 48 0 24 24 0 1 1 -48 0zm56 232a24 24 0 1 1 0 48 24 24 0 1 1 0-48z";
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
    exports.faTreeDecorated = exports.definition;
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

// src/_entries/faTreeDecorated.js
var import_faTreeDecorated = __toESM(require_faTreeDecorated());
var export_faTreeDecorated = import_faTreeDecorated.definition;
export {
  export_faTreeDecorated as faTreeDecorated
};
