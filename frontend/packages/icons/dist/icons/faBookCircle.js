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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCircleBookOpen.js
var require_faCircleBookOpen = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCircleBookOpen.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "circle-book-open";
    var width = 512;
    var height = 512;
    var aliases = ["book-circle"];
    var unicode = "e0ff";
    var svgPathData = "M256 512A256 256 0 1 0 256 0a256 256 0 1 0 0 512zM184 336c-23.1 0-41 11.1-46.3 14.8c-1.1 .8-2.4 1.2-3.7 1.2c-3.3 0-5.9-2.7-5.9-5.9l0-160.7c0-5.8 3.1-11.1 8.3-13.5c10.4-4.7 29.1-11.9 47.7-11.9s37.2 7.1 47.7 11.9c5.2 2.4 8.3 7.7 8.3 13.5l0 160.7c0 3.3-2.7 5.9-5.9 5.9c-1.3 0-2.6-.4-3.7-1.2C225 347.1 207.1 336 184 336zm144 0c-23.1 0-41 11.1-46.3 14.8c-1.1 .8-2.4 1.2-3.7 1.2c-3.3 0-5.9-2.7-5.9-5.9l0-160.7c0-5.8 3.1-11.1 8.3-13.5c10.4-4.7 29.1-11.9 47.7-11.9s37.2 7.1 47.7 11.9c5.2 2.4 8.3 7.7 8.3 13.5l0 160.7c0 3.3-2.7 5.9-5.9 5.9c-1.3 0-2.6-.4-3.7-1.2C369 347.1 351.1 336 328 336z";
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
    exports.faCircleBookOpen = exports.definition;
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

// src/_entries/faBookCircle.js
var import_faCircleBookOpen = __toESM(require_faCircleBookOpen());
var export_faBookCircle = import_faCircleBookOpen.definition;
export {
  export_faBookCircle as faBookCircle
};
