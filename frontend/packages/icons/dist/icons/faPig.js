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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPig.js
var require_faPig = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPig.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "pig";
    var width = 640;
    var height = 512;
    var aliases = [128022];
    var unicode = "f706";
    var svgPathData = "M513.5 32c8 0 14.5 6.5 14.5 14.5L528 78c0 19.7-9 37.8-23.6 49.7c6.4 4.9 12.7 10.1 18.6 15.7c17.6-15.9 28.5-38.6 29-63.3l40 0c8.8 0 16 7.2 16 16l0 16c0 32.8-24.6 59.8-56.3 63.5l6.8 9C570 200 585 212.5 602.2 221.1L608 224l8 0c13.3 0 24 10.7 24 24l0 96c0 13.3-10.7 24-24 24l-43.5 0c-22.2 21.2-49.7 36.2-79.8 43.3l-13 45.5C475.8 470.5 463.3 480 449 480l-33 0c-17.7 0-32-14.3-32-32l0-32-64 0-48 0 0 32c0 17.7-14.3 32-32 32l-33 0c-14.3 0-26.8-9.5-30.8-23.2L155 382.4C110.3 358 80 310.5 80 256l0-16-32.6 0C21.2 240 0 218.8 0 192.6c0-21.8 14.8-40.7 35.9-46l8.2-2c8.6-2.1 17.3 3.1 19.4 11.6s-3.1 17.3-11.6 19.4l-8.2 2c-6.9 1.7-11.7 7.9-11.7 15c0 8.5 6.9 15.4 15.4 15.4l35.8 0C98 135 162.6 80 240 80l122.4 0c14.5 0 28.8 1.3 42.7 3.9c8.1-19.5 25.4-34.4 47-39.1l58.4-12.5c1-.2 2-.3 3-.3zM468 256a20 20 0 1 0 0-40 20 20 0 1 0 0 40z";
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
    exports.faPig = exports.definition;
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

// src/_entries/faPig.js
var import_faPig = __toESM(require_faPig());
var export_faPig = import_faPig.definition;
export {
  export_faPig as faPig
};
