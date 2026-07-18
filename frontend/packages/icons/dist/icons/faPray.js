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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPersonPraying.js
var require_faPersonPraying = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPersonPraying.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "person-praying";
    var width = 384;
    var height = 512;
    var aliases = [128720, "pray"];
    var unicode = "f683";
    var svgPathData = "M320 64a64 64 0 1 0 -128 0 64 64 0 1 0 128 0zM200.7 264l22.9 31.5c6.5 8.9 16.3 14.7 27.2 16.1s21.9-1.7 30.4-8.7l88-72c17.1-14 19.6-39.2 5.6-56.3s-39.2-19.6-56.3-5.6l-55.2 45.2-26.2-36c-15.6-21.5-40.6-34.2-67.2-34.2-30.9 0-59.2 17.1-73.6 44.4L47.8 280.9c-20.2 38.5-9.4 85.9 25.6 111.8L126.6 432 40 432c-22.1 0-40 17.9-40 40s17.9 40 40 40l208 0c17.3 0 32.6-11.1 38-27.5s-.3-34.4-14.2-44.7l-116.1-85.8 45-90z";
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
    exports.faPersonPraying = exports.definition;
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

// src/_entries/faPray.js
var import_faPersonPraying = __toESM(require_faPersonPraying());
var export_faPray = import_faPersonPraying.definition;
export {
  export_faPray as faPray
};
