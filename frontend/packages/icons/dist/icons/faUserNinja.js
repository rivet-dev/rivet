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

// src/node_modules/@fortawesome/free-solid-svg-icons/faUserNinja.js
var require_faUserNinja = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faUserNinja.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-ninja";
    var width = 448;
    var height = 512;
    var aliases = [129399];
    var unicode = "f504";
    var svgPathData = "M352 128c0 70.7-57.3 128-128 128-57.2 0-105.6-37.5-122-89.3-1.1 1.3-2.2 2.6-3.5 3.8-15.8 15.8-38.8 20.7-53.6 22.1-8.1 .8-14.6-5.7-13.8-13.8 1.4-14.7 6.3-37.8 22.1-53.6 5.8-5.8 12.6-10.1 19.6-13.4-7-3.2-13.8-7.6-19.6-13.4-15.8-15.8-20.7-38.8-22.1-53.6-.8-8.1 5.7-14.6 13.8-13.8 14.7 1.4 37.8 6.3 53.6 22.1 4.8 4.8 8.7 10.4 11.7 16.1 21.3-41.2 64.3-69.4 113.8-69.4 70.7 0 128 57.3 128 128zM144 112c0 8.8 7.2 16 16 16l128 0c8.8 0 16-7.2 16-16s-7.2-16-16-16L160 96c-8.8 0-16 7.2-16 16zm-1.4 211l67 50.2c8.5 6.4 20.3 6.4 28.8 0l67-50.2c6.5-4.9 15.2-6.2 22.6-2.8 61.4 28.2 104.1 90.2 104.1 162.1 0 16.4-13.3 29.7-29.7 29.7L45.7 512c-16.4 0-29.7-13.3-29.7-29.7 0-72 42.7-134 104.1-162.1 7.4-3.4 16.1-2.1 22.6 2.8z";
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
    exports.faUserNinja = exports.definition;
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

// src/_entries/faUserNinja.js
var import_faUserNinja = __toESM(require_faUserNinja());
var export_faUserNinja = import_faUserNinja.definition;
export {
  export_faUserNinja as faUserNinja
};
