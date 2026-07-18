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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPeach.js
var require_faPeach = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPeach.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "peach";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e20b";
    var svgPathData = "M344 80c-31.5 0-61.4 7.3-88 20.4C229.4 87.3 199.5 80 168 80c-11.4 0-22.6 1-33.5 2.8C120.5 67.2 112 46.6 112 24c0-13.3 10.7-24 24-24l24 0c53 0 96 43 96 96c0-53 43-96 96-96l24 0c13.3 0 24 10.7 24 24c0 22.6-8.5 43.2-22.5 58.8C366.6 81 355.4 80 344 80zM168 112c17.8 0 34.9 2.8 50.9 7.9c63 20 109.9 76.2 116.3 144.2c.8 8.8 7.9 16 16.8 16s16.1-7.2 15.4-16c-4.6-58.2-34.2-109.5-78-143c.8-.2 1.5-.5 2.3-.7c16.5-5.4 34-8.3 52.3-8.3c92.8 0 168 75.2 168 168c0 136.2-199.3 214.6-240.6 229.4c-5 1.8-10.1 2.6-15.4 2.6s-10.4-.8-15.4-2.6C199.3 494.6 0 416.2 0 280c0-92.8 75.2-168 168-168z";
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
    exports.faPeach = exports.definition;
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

// src/_entries/faPeach.js
var import_faPeach = __toESM(require_faPeach());
var export_faPeach = import_faPeach.definition;
export {
  export_faPeach as faPeach
};
