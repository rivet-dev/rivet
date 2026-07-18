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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCandyCorn.js
var require_faCandyCorn = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCandyCorn.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "candy-corn";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f6bd";
    var svgPathData = "M440.6 256c9.8 25.6 19.6 49 28.7 67.2c3.3 6.7 6.2 12.4 9.3 17.7c.5 .8 1 1.7 1.4 2.4c.5-.8 1-1.6 1.4-2.4c3.1-5.3 6-11 9.3-17.7c9-18.2 18.9-41.5 28.7-67.2l-78.9 0zM480 0c44.1 0 90.4 4.2 132.6 18.1C629 23.5 640 38.8 640 56c0 17.4-5.3 43.8-12.3 71.4c-4.4-1-9.3-2.1-14.5-3.1c-30.9-6.2-76-12.3-133.1-12.3s-102.2 6.1-133.1 12.3c-5.3 1.1-10.1 2.1-14.5 3.1c-7-27.6-12.3-54-12.3-71.4c0-17.2 11.4-32.6 27.8-38.1C390 4.4 436 0 480 0zM370.3 250.4c-11-30.8-21.3-62.9-29.6-92.1c3.8-.9 8-1.8 12.4-2.7c29.1-5.8 72-11.7 126.9-11.7s97.8 5.9 126.9 11.7c4.5 .9 8.6 1.8 12.4 2.7c-8.3 29.2-18.6 61.3-29.6 92.1c-13.2 37.2-27.9 73.6-41.6 101.3c-13.4 27-32.9 64.3-68.1 64.3s-54.6-37.3-68.1-64.3c-13.8-27.7-28.4-64.1-41.6-101.3zM83 211.1c30.4-30.4 65.3-59.4 103.9-79c15-7.5 33.1-4.6 45 7.3c12 12 26.6 33.9 40.7 57.7c-3.8 2.3-7.8 5-12.2 7.9c-25.6 17.1-60.9 43.9-100.3 83.3S94 363 76.9 388.6c-2.9 4.3-5.5 8.4-7.9 12.2C45.2 386.6 23.3 372 11.3 360C-.6 348.1-3.3 329.6 4.2 314.6C23.9 276.2 52.6 241.5 83 211.1zm97 248.2c-28.8-13.7-58-28.7-83.9-43c2-3.2 4.3-6.7 6.7-10.4c16-24 41.6-57.7 79.4-95.5S253.7 247 277.7 231c3.7-2.5 7.2-4.7 10.4-6.7c14.4 25.9 29.4 55.1 43.1 83.9c16.5 34.8 31.5 70 41.1 98.5c9.3 27.9 21.7 67-2.6 91.2s-63.4 11.9-91.2 2.6c-28.5-9.6-63.7-24.6-98.5-41.1zm52.3-44.6c24.4 11 47.3 20.2 66.1 26.5c6.9 2.3 12.8 4.2 18.6 5.8c.9 .2 1.8 .5 2.7 .7c-.2-.9-.4-1.7-.7-2.7c-1.5-5.8-3.4-11.7-5.8-18.6c-6.3-18.8-15.6-41.7-26.5-66.1l-54.4 54.4z";
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
    exports.faCandyCorn = exports.definition;
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

// src/_entries/faCandyCorn.js
var import_faCandyCorn = __toESM(require_faCandyCorn());
var export_faCandyCorn = import_faCandyCorn.definition;
export {
  export_faCandyCorn as faCandyCorn
};
