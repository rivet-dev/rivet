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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCoffeeBeans.js
var require_faCoffeeBeans = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCoffeeBeans.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "coffee-beans";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e13f";
    var svgPathData = "M507.5 209.7c-3.4 12.3-19.1 11.8-25.4 .7c-5.4-9.5-11.3-18.5-17.6-26c-27.6-32.6-60.8-44.5-90.1-55l-1-.3c-30-10.7-55.8-20.2-77.5-45.9c-10.8-12.8-21.3-33.4-29.3-52c-1.7-4-.6-8.7 3-11.2C320.9-16 401.3-2.6 458 54c45.7 45.7 63.2 106.7 49.5 155.6zM228.5 78.3c3.4-12.3 19.1-11.8 25.4-.7c5.3 9.5 11.3 18.5 17.6 26c27.6 32.6 60.8 44.5 90.1 55l1 .3c30 10.7 55.8 20.2 77.5 45.9c10.8 12.8 21.3 33.4 29.3 52c1.7 4 .6 8.7-3 11.2C415.1 304 334.7 290.6 278 234c-45.7-45.7-63.2-106.7-49.5-155.6zM75.5 206c11.2-6.3 22 5.2 18.5 17.6c-2.9 10.5-5.2 21.2-6 31c-3.6 42.8 11.6 74.9 25 103.2l.4 .9c13.7 28.9 25.3 54.1 22.5 87.8c-1.4 16.7-8.6 38.9-16.1 57.9c-1.6 4.1-5.8 6.6-10.1 5.8C47.7 499.3 0 432.6 0 352c0-65 31-120.9 75.5-146zM180.5 498c-11.2 6.3-22-5.2-18.5-17.6c2.9-10.5 5.2-21.2 6-31c3.6-42.8-11.6-74.9-25-103.2l-.4-.9c-13.7-28.9-25.3-54.1-22.5-87.8c1.4-16.7 8.6-38.9 16.1-57.9c1.6-4.1 5.8-6.6 10.1-5.8C208.3 204.7 256 271.4 256 352c0 65-31 120.9-75.5 146z";
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
    exports.faCoffeeBeans = exports.definition;
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

// src/_entries/faCoffeeBeans.js
var import_faCoffeeBeans = __toESM(require_faCoffeeBeans());
var export_faCoffeeBeans = import_faCoffeeBeans.definition;
export {
  export_faCoffeeBeans as faCoffeeBeans
};
