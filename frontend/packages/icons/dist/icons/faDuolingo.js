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

// src/node_modules/@fortawesome/free-brands-svg-icons/faDuolingo.js
var require_faDuolingo = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faDuolingo.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "duolingo";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e812";
    var svgPathData = "M504.3 230.5c10.4 88.5-43.4 172.1-128.2 199s-176.6-10.3-218.9-88.7c-72 3-137.4-39.8-163.2-105.5-2.6-6.6-1.9-14 1.9-19.9s10.2-9.7 17.2-10.1l96.1-6-23.8-75.9c-6.3-20-2.2-41.9 11-58.3s33.7-25 54.5-23c52.2 5 86.8 4.8 104-.6S300 16.7 339-16.9c15.9-13.7 37.7-18.5 57.8-12.7s36.1 21.3 42.4 41.4L462.8 87 543.6 37.3c6-3.7 13.4-4.3 19.9-1.6s11.3 8.3 12.9 15.2c16.2 68.2-12.4 140.3-72.1 179.5zM258.5 475.9l0 0c15.1 7.5 21.5 25.8 14.2 41s-25.4 21.9-40.7 14.9l-43.9-21c-15.1-7.5-21.5-25.8-14.2-41s25.4-21.9 40.7-14.9l43.8 21zM564.6 345.7c14.8 8.4 20 27.3 11.6 42.2l-23.9 42.4c-8.7 14.3-27.1 19.2-41.6 10.9s-19.9-26.6-12-41.4l23.9-42.4c4-7.1 10.7-12.4 18.6-14.6s16.3-1.2 23.4 2.9z";
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
    exports.faDuolingo = exports.definition;
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

// src/_entries/faDuolingo.js
var import_faDuolingo = __toESM(require_faDuolingo());
var export_faDuolingo = import_faDuolingo.definition;
export {
  export_faDuolingo as faDuolingo
};
