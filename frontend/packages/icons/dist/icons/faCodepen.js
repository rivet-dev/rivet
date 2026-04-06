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

// src/node_modules/@fortawesome/free-brands-svg-icons/faCodepen.js
var require_faCodepen = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faCodepen.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "codepen";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f1cb";
    var svgPathData = "M502.3 159.7l-234-156c-8-4.9-16.5-5-24.6 0l-234 156c-6 4-9.7 11.1-9.7 18.3L0 334c0 7.1 3.7 14.3 9.7 18.3l234 156c8 4.9 16.5 5 24.6 0l234-156c6-4 9.7-11.1 9.7-18.3l0-156c0-7.1-3.7-14.3-9.7-18.3zM278 63.1l172.3 114.9-76.9 51.4-95.4-63.7 0-102.6zm-44 0L234 165.7 138.6 229.4 61.7 178 234 63.1zM44 219.1L99.1 256 44 292.8 44 219.1zM234 448.8L61.7 334 138.6 282.6 234 346.3 234 448.8zM256 308l-77.7-52 77.7-52 77.7 52-77.7 52zm22 140.9l0-102.6 95.4-63.7 76.9 51.4-172.3 114.9zm190-156l-55.1-36.9 55.1-36.9 0 73.7z";
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
    exports.faCodepen = exports.definition;
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

// src/_entries/faCodepen.js
var import_faCodepen = __toESM(require_faCodepen());
var export_faCodepen = import_faCodepen.definition;
export {
  export_faCodepen as faCodepen
};
